//! Internal module graph bookkeeping.
//! 内部模块图状态管理。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::module_loader::ModuleInfo;
use crate::module_paths::ModulePathResolver;
use crate::{Import, ModuleId, ModulePath};

/// Internal graph state shared by the compatibility loader.
/// 兼容加载器共享的内部模块图状态。
#[derive(Debug, Clone, Default)]
pub(crate) struct ModuleGraphState {
    modules: HashMap<ModuleId, ModuleInfo>,
    path_to_id: HashMap<Vec<String>, ModuleId>,
    file_to_id: HashMap<PathBuf, ModuleId>,
    load_order: Vec<ModuleId>,
    dependents: HashMap<ModuleId, HashSet<ModuleId>>,
}

impl ModuleGraphState {
    /// Look up a module by path.
    /// 按路径查找模块。
    pub(crate) fn lookup_module(&self, path: &[String]) -> Option<ModuleId> {
        self.path_to_id.get(path).copied()
    }

    /// Look up a module by file path.
    /// 按文件路径查找模块。
    pub(crate) fn module_id_for_file(&self, file_path: &Path) -> Option<ModuleId> {
        self.file_to_id.get(file_path).copied()
    }

    /// Get module info by ID.
    /// 按 ID 获取模块信息。
    pub(crate) fn get_module(&self, id: ModuleId) -> Option<&ModuleInfo> {
        self.modules.get(&id)
    }

    /// Get mutable module info by ID.
    /// 按 ID 获取可变模块信息。
    pub(crate) fn get_module_mut(&mut self, id: ModuleId) -> Option<&mut ModuleInfo> {
        self.modules.get_mut(&id)
    }

    /// Iterate all loaded modules.
    /// 迭代所有已加载模块。
    pub(crate) fn all_modules(&self) -> impl Iterator<Item = (&Vec<String>, &ModuleInfo)> {
        self.path_to_id
            .iter()
            .filter_map(|(path, &id)| self.modules.get(&id).map(|info| (path, info)))
    }

    /// Return module load order.
    /// 返回模块加载顺序。
    pub(crate) fn load_order(&self) -> &[ModuleId] {
        &self.load_order
    }

    /// Find the parent module for a path.
    /// 查找模块路径对应的父模块。
    pub(crate) fn find_parent_module(&self, path: &[String]) -> Option<ModuleId> {
        if path.len() <= 1 {
            return None;
        }
        self.path_to_id.get(&path[..path.len() - 1]).copied()
    }

    /// Collect imported module dependencies for a module path.
    /// 收集指定模块路径的导入模块依赖。
    pub(crate) fn collect_dependencies(
        &self,
        imports: &[Import],
        from_module: &[String],
        path_resolver: &ModulePathResolver,
    ) -> Vec<ModuleId> {
        let mut deps = HashSet::new();
        for import in imports {
            let import_path = ModulePath::from_hir_import(import);
            if let Some(abs_path) =
                path_resolver.resolve_module_path(&import_path, Some(from_module))
            {
                if abs_path == from_module {
                    continue;
                }
                if let Some(dep_id) = self.lookup_module(&abs_path) {
                    deps.insert(dep_id);
                }
            }
        }
        deps.into_iter().collect()
    }

    /// Register a loaded module.
    /// 注册已加载模块。
    pub(crate) fn register_module(&mut self, info: ModuleInfo) {
        let module_id = info.id;
        let path = info.path.clone();
        let file_path = info.file_path.clone();
        self.modules.insert(module_id, info);
        self.path_to_id.insert(path, module_id);
        self.file_to_id.insert(file_path, module_id);
        self.load_order.push(module_id);
    }

    /// Register reverse dependency edges for a module.
    /// 为模块注册反向依赖边。
    pub(crate) fn register_dependency_edges(
        &mut self,
        module_id: ModuleId,
        dependencies: &[ModuleId],
    ) {
        for dep in dependencies {
            self.dependents.entry(*dep).or_default().insert(module_id);
        }
    }

    /// Register a child under an already-loaded parent.
    /// 在已加载的父模块下登记子模块。
    pub(crate) fn register_child(&mut self, parent_id: ModuleId, child_id: ModuleId) {
        if let Some(parent_info) = self.modules.get_mut(&parent_id) {
            parent_info.children.push(child_id);
        }
    }

    /// Compute reverse-dependent closure from a starting module.
    /// 计算从起始模块出发的反向依赖闭包。
    pub(crate) fn dependent_closure(&self, module_id: ModuleId) -> Vec<ModuleId> {
        let mut stack = vec![module_id];
        let mut seen = HashSet::new();
        let mut ordered = Vec::new();

        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            ordered.push(current);
            if let Some(deps) = self.dependents.get(&current) {
                stack.extend(deps.iter().copied());
            }
        }

        ordered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Visibility;

    fn info(id: u32, path: &[&str], file: &str, parent: Option<ModuleId>) -> ModuleInfo {
        ModuleInfo {
            id: ModuleId(id),
            path: path.iter().map(|segment| (*segment).to_string()).collect(),
            file_path: PathBuf::from(file),
            parent,
            children: Vec::new(),
            dependencies: Vec::new(),
            exports: HashMap::new(),
            items: HashMap::from([(
                String::from("dummy"),
                (crate::DefId(0), Visibility::Private),
            )]),
            mtime: None,
        }
    }

    #[test]
    fn graph_registers_parent_child_and_lookup_state() {
        let mut graph = ModuleGraphState::default();
        graph.register_module(info(0, &["root"], "/tmp/root.neve", None));

        let parent = graph.find_parent_module(&["root".into(), "child".into()]);
        assert_eq!(parent, Some(ModuleId(0)));

        graph.register_module(info(1, &["root", "child"], "/tmp/root/child.neve", parent));
        graph.register_child(ModuleId(0), ModuleId(1));

        assert_eq!(graph.lookup_module(&["root".into()]), Some(ModuleId(0)));
        assert_eq!(
            graph.module_id_for_file(Path::new("/tmp/root/child.neve")),
            Some(ModuleId(1))
        );
        assert_eq!(
            graph.get_module(ModuleId(0)).unwrap().children,
            vec![ModuleId(1)]
        );
    }

    #[test]
    fn graph_tracks_reverse_dependents() {
        let mut graph = ModuleGraphState::default();
        graph.register_module(info(0, &["a"], "/tmp/a.neve", None));
        graph.register_module(info(1, &["b"], "/tmp/b.neve", None));
        graph.register_module(info(2, &["c"], "/tmp/c.neve", None));

        graph.register_dependency_edges(ModuleId(1), &[ModuleId(0)]);
        graph.register_dependency_edges(ModuleId(2), &[ModuleId(1)]);

        let closure = graph.dependent_closure(ModuleId(0));
        assert!(closure.contains(&ModuleId(0)));
        assert!(closure.contains(&ModuleId(1)));
        assert!(closure.contains(&ModuleId(2)));
    }
}
