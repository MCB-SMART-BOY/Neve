//! Internal per-module lowering helpers for the compatibility loader.
//! 兼容加载器使用的模块级降级辅助逻辑。

use std::collections::{HashMap, HashSet};

use neve_syntax::{self, SourceFile, Visibility};

use crate::{DefId, Import, ImportKind, Module, ModuleId, Resolver};

/// Lowered artifacts produced for a single module load.
/// 单个模块加载阶段产出的降级结果。
#[derive(Debug, Clone)]
pub(crate) struct LoweredModuleArtifacts {
    pub(crate) module: Module,
    pub(crate) items: HashMap<String, (DefId, Visibility)>,
    pub(crate) exports: HashMap<String, DefId>,
    pub(crate) next_def_id: u32,
}

/// Collect all imports from a source file.
/// 从源文件收集所有导入。
pub(crate) fn collect_imports(file: &SourceFile) -> Vec<Import> {
    file.items
        .iter()
        .filter_map(|item| match &item.kind {
            neve_syntax::ItemKind::Import(import_def) => {
                let prefix = match import_def.prefix {
                    neve_syntax::PathPrefix::Absolute => crate::ImportPathPrefix::Absolute,
                    neve_syntax::PathPrefix::Self_ => crate::ImportPathPrefix::Self_,
                    neve_syntax::PathPrefix::Super => crate::ImportPathPrefix::Super,
                    neve_syntax::PathPrefix::Crate => crate::ImportPathPrefix::Crate,
                };

                let path: Vec<String> = import_def.path.iter().map(|p| p.name.clone()).collect();

                let kind = match &import_def.items {
                    neve_syntax::ImportItems::Module => ImportKind::Module,
                    neve_syntax::ImportItems::Items(items) => {
                        ImportKind::Items(items.iter().map(|i| i.name.clone()).collect())
                    }
                    neve_syntax::ImportItems::All => ImportKind::All,
                };

                let alias = import_def.alias.as_ref().map(|a| a.name.clone());

                Some(Import {
                    prefix,
                    path,
                    kind,
                    alias,
                    is_pub: import_def.visibility == Visibility::Public,
                    span: item.span,
                })
            }
            _ => None,
        })
        .collect()
}

/// Lower a source file after imports have already been resolved.
/// 在导入已经解析完成后对源文件执行降级。
pub(crate) fn lower_module_with_imports(
    source_file: &SourceFile,
    path: &[String],
    module_id: ModuleId,
    next_def_id: u32,
    resolved_imports: Vec<(String, DefId)>,
    reexports: Vec<(String, DefId)>,
) -> LoweredModuleArtifacts {
    let mut resolver = Resolver::new();
    resolver.set_def_id_counter(next_def_id);
    resolver.register_imports(resolved_imports);

    let module_name = path.last().cloned().unwrap_or_else(|| "main".to_string());
    let module =
        resolver.resolve_with_path_and_id(source_file, module_name, path.to_vec(), module_id);
    let next_def_id = resolver.next_def_id();

    let export_names: HashSet<String> = module
        .exports
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();
    let mut items = HashMap::new();
    let mut exports = HashMap::new();

    for (name, def_id) in resolver.global_defs() {
        let visibility = if export_names.contains(name) {
            Visibility::Public
        } else {
            Visibility::Private
        };

        items.insert(name.clone(), (*def_id, visibility));
        if visibility == Visibility::Public {
            exports.insert(name.clone(), *def_id);
        }
    }

    for (name, def_id) in reexports {
        if items.contains_key(&name) {
            continue;
        }
        items.insert(name.clone(), (def_id, Visibility::Public));
        exports.insert(name, def_id);
    }

    LoweredModuleArtifacts {
        module,
        items,
        exports,
        next_def_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neve_parser::parse;

    #[test]
    fn collect_imports_preserves_prefix_kind_and_visibility() {
        let source = r#"
            use std.list (map);
            use self.utils = utils;
        "#;
        let (file, diagnostics) = parse(source);
        assert!(diagnostics.is_empty());

        let imports = collect_imports(&file);
        assert_eq!(imports.len(), 2);
        // v4.0: all imports are public by default
        assert!(imports[0].is_pub);
        assert!(matches!(imports[0].kind, ImportKind::Items(_)));
        assert!(matches!(
            imports[0].prefix,
            crate::ImportPathPrefix::Absolute
        ));
        assert_eq!(imports[0].path, vec!["std", "list"]);

        assert!(imports[1].is_pub);
        assert!(matches!(imports[1].kind, ImportKind::Module));
        assert!(matches!(imports[1].prefix, crate::ImportPathPrefix::Self_));
        assert_eq!(imports[1].alias.as_deref(), Some("utils"));
    }

    #[test]
    fn lower_module_includes_public_defs_and_reexports() {
        let source = r#"
            fn local() = 1;
            fn hidden() = 2;
        "#;
        let (file, diagnostics) = parse(source);
        assert!(diagnostics.is_empty());

        let lowered = lower_module_with_imports(
            &file,
            &["main".into()],
            ModuleId(0),
            0,
            Vec::new(),
            vec![(String::from("external"), DefId(42))],
        );

        assert!(lowered.items.contains_key("local"));
        assert!(lowered.items.contains_key("hidden"));
        assert_eq!(lowered.exports.get("external"), Some(&DefId(42)));
        // v4.0: all items are public by default
        assert!(lowered.exports.contains_key("local"));
        assert!(lowered.exports.contains_key("hidden"));
    }
}
