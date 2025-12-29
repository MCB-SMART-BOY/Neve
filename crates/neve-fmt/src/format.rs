//! AST formatter.
//! AST 格式化器。
//!
//! Provides the main formatting logic for converting AST nodes back to
//! properly formatted source code.
//! 提供将 AST 节点转换回正确格式化源代码的主要格式化逻辑。

use crate::config::FormatConfig;
use crate::printer::Printer;
use neve_syntax::{
    BinOp, EnumDef, Expr, ExprKind, FieldDef, FnDef, Generator, GenericParam, ImplDef, ImplItem,
    ImportDef, ImportItems, Item, ItemKind, LambdaParam, LetDef, LiteralPattern, MatchArm, Param,
    Pattern, PatternKind, RecordField, RecordPatternField, RecordTypeField, SourceFile, Stmt,
    StmtKind, StringPart, StructDef, TraitDef, TraitItem, Type, TypeAlias, TypeKind, UnaryOp,
    VariantKind, Visibility,
};

/// Code formatter.
/// 代码格式化器。
pub struct Formatter {
    /// Formatting configuration. / 格式化配置。
    config: FormatConfig,
}

impl Formatter {
    /// Create a new formatter.
    /// 创建新的格式化器。
    pub fn new(config: FormatConfig) -> Self {
        Self { config }
    }

    /// Format a source file.
    /// 格式化源文件。
    pub fn format(&self, file: &SourceFile) -> String {
        let mut printer = Printer::new(self.config.clone());

        for (i, item) in file.items.iter().enumerate() {
            if i > 0 {
                // Add extra blank line between top-level items if configured
                // 如果配置了，在顶级项之间添加额外的空行
                if printer.config().blank_lines_between_items {
                    printer.newline();
                }
                printer.newline();
            }
            self.format_item(&mut printer, item);
        }

        // Ensure we're at indent level 0 at end of file
        // 确保在文件末尾缩进级别为 0
        debug_assert_eq!(printer.current_indent(), 0, "unbalanced indentation");

        printer.finish()
    }

    /// Format an item.
    /// 格式化项。
    fn format_item(&self, p: &mut Printer, item: &Item) {
        match &item.kind {
            ItemKind::Let(def) => self.format_let(p, def),
            ItemKind::Fn(def) => self.format_fn(p, def),
            ItemKind::TypeAlias(def) => self.format_type_alias(p, def),
            ItemKind::Struct(def) => self.format_struct(p, def),
            ItemKind::Enum(def) => self.format_enum(p, def),
            ItemKind::Trait(def) => self.format_trait(p, def),
            ItemKind::Impl(def) => self.format_impl(p, def),
            ItemKind::Import(def) => self.format_import(p, def),
        }
    }

    /// Format a let binding.
    /// 格式化 let 绑定。
    fn format_let(&self, p: &mut Printer, def: &LetDef) {
        if def.visibility == Visibility::Public {
            p.write("pub ");
        }
        p.write("let ");
        self.format_pattern(p, &def.pattern);

        if let Some(ref ty) = def.ty {
            p.write(": ");
            self.format_type(p, ty);
        }

        p.write(" = ");
        self.format_expr(p, &def.value);
        p.write(";");
        p.newline();
    }

    /// Format a function definition.
    /// 格式化函数定义。
    fn format_fn(&self, p: &mut Printer, def: &FnDef) {
        if def.visibility == Visibility::Public {
            p.write("pub ");
        }
        p.write("fn ");
        p.write(&def.name.name);

        // Generics / 泛型
        self.format_generics(p, &def.generics);

        // Parameters / 参数
        p.write("(");
        for (i, param) in def.params.iter().enumerate() {
            if i > 0 {
                p.write(", ");
            }
            self.format_param(p, param);
        }
        p.write(")");

        // Return type / 返回类型
        if let Some(ref ret_ty) = def.return_type {
            p.write(" -> ");
            self.format_type(p, ret_ty);
        }

        // Body / 函数体
        p.write(" = ");
        self.format_expr(p, &def.body);
        p.write(";");
        p.newline();
    }

    /// Format a type alias.
    /// 格式化类型别名。
    fn format_type_alias(&self, p: &mut Printer, def: &TypeAlias) {
        if def.visibility == Visibility::Public {
            p.write("pub ");
        }
        p.write("type ");
        p.write(&def.name.name);
        self.format_generics(p, &def.generics);
        p.write(" = ");
        self.format_type(p, &def.ty);
        p.write(";");
        p.newline();
    }

    /// Format a struct definition.
    /// 格式化结构体定义。
    fn format_struct(&self, p: &mut Printer, def: &StructDef) {
        if def.visibility == Visibility::Public {
            p.write("pub ");
        }
        p.write("struct ");
        p.write(&def.name.name);
        self.format_generics(p, &def.generics);

        if def.fields.is_empty() {
            p.write(";");
        } else {
            p.write(" {");
            p.newline();
            p.indent();
            for field in &def.fields {
                self.format_field_def(p, field);
                p.write(",");
                p.newline();
            }
            p.dedent();
            p.write("}");
        }
        p.newline();
    }

    /// Format an enum definition.
    /// 格式化枚举定义。
    fn format_enum(&self, p: &mut Printer, def: &EnumDef) {
        if def.visibility == Visibility::Public {
            p.write("pub ");
        }
        p.write("enum ");
        p.write(&def.name.name);
        self.format_generics(p, &def.generics);
        p.write(" {");
        p.newline();
        p.indent();

        for variant in &def.variants {
            p.write(&variant.name.name);
            match &variant.kind {
                VariantKind::Unit => {}
                VariantKind::Tuple(types) => {
                    p.write("(");
                    for (i, ty) in types.iter().enumerate() {
                        if i > 0 {
                            p.write(", ");
                        }
                        self.format_type(p, ty);
                    }
                    p.write(")");
                }
                VariantKind::Record(fields) => {
                    p.write(" #{ ");
                    for (i, field) in fields.iter().enumerate() {
                        if i > 0 {
                            p.write(", ");
                        }
                        self.format_field_def(p, field);
                    }
                    p.write(" }");
                }
            }
            p.write(",");
            p.newline();
        }

        p.dedent();
        p.write("}");
        p.newline();
    }

    /// Format a trait definition.
    /// 格式化 trait 定义。
    fn format_trait(&self, p: &mut Printer, def: &TraitDef) {
        if def.visibility == Visibility::Public {
            p.write("pub ");
        }
        p.write("trait ");
        p.write(&def.name.name);
        self.format_generics(p, &def.generics);
        p.write(" {");
        p.newline();
        p.indent();

        for item in &def.items {
            self.format_trait_item(p, item);
        }

        p.dedent();
        p.write("}");
        p.newline();
    }

    /// Format an impl block.
    /// 格式化 impl 块。
    fn format_impl(&self, p: &mut Printer, def: &ImplDef) {
        p.write("impl");
        self.format_generics(p, &def.generics);
        p.write(" ");

        if let Some(ref trait_) = def.trait_ {
            self.format_type(p, trait_);
            p.write(" for ");
        }

        self.format_type(p, &def.target);
        p.write(" {");
        p.newline();
        p.indent();

        for item in &def.items {
            self.format_impl_item(p, item);
        }

        p.dedent();
        p.write("}");
        p.newline();
    }

    /// Format an import.
    /// 格式化导入。
    fn format_import(&self, p: &mut Printer, def: &ImportDef) {
        p.write("import ");
        for (i, part) in def.path.iter().enumerate() {
            if i > 0 {
                p.write(".");
            }
            p.write(&part.name);
        }

        match &def.items {
            ImportItems::Module => {}
            ImportItems::Items(items) => {
                p.write(" (");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    p.write(&item.name);
                }
                p.write(")");
            }
            ImportItems::All => {
                p.write(" (*)");
            }
        }

        if let Some(ref alias) = def.alias {
            p.write(" as ");
            p.write(&alias.name);
        }

        p.write(";");
        p.newline();
    }

    /// Format generics.
    /// 格式化泛型。
    fn format_generics(&self, p: &mut Printer, generics: &[GenericParam]) {
        if !generics.is_empty() {
            p.write("<");
            for (i, param) in generics.iter().enumerate() {
                if i > 0 {
                    p.write(", ");
                }
                p.write(&param.name.name);
                if !param.bounds.is_empty() {
                    p.write(": ");
                    for (j, bound) in param.bounds.iter().enumerate() {
                        if j > 0 {
                            p.write(" + ");
                        }
                        self.format_type(p, bound);
                    }
                }
            }
            p.write(">");
        }
    }

    /// Format a function parameter.
    /// 格式化函数参数。
    fn format_param(&self, p: &mut Printer, param: &Param) {
        if param.is_lazy {
            p.write("lazy ");
        }
        self.format_pattern(p, &param.pattern);
        p.write(": ");
        self.format_type(p, &param.ty);
    }

    /// Format a field definition.
    /// 格式化字段定义。
    fn format_field_def(&self, p: &mut Printer, field: &FieldDef) {
        p.write(&field.name.name);
        p.write(": ");
        self.format_type(p, &field.ty);
        if let Some(ref default) = field.default {
            p.write(" = ");
            self.format_expr(p, default);
        }
    }

    /// Format a trait item.
    /// 格式化 trait 项。
    fn format_trait_item(&self, p: &mut Printer, item: &TraitItem) {
        p.write("fn ");
        p.write(&item.name.name);
        self.format_generics(p, &item.generics);
        p.write("(");
        for (i, param) in item.params.iter().enumerate() {
            if i > 0 {
                p.write(", ");
            }
            self.format_param(p, param);
        }
        p.write(")");

        if let Some(ref ret_ty) = item.return_type {
            p.write(" -> ");
            self.format_type(p, ret_ty);
        }

        if let Some(ref default) = item.default {
            p.write(" = ");
            self.format_expr(p, default);
        }

        p.write(";");
        p.newline();
    }

    /// Format an impl item.
    /// 格式化 impl 项。
    fn format_impl_item(&self, p: &mut Printer, item: &ImplItem) {
        p.write("fn ");
        p.write(&item.name.name);
        self.format_generics(p, &item.generics);
        p.write("(");
        for (i, param) in item.params.iter().enumerate() {
            if i > 0 {
                p.write(", ");
            }
            self.format_param(p, param);
        }
        p.write(")");

        if let Some(ref ret_ty) = item.return_type {
            p.write(" -> ");
            self.format_type(p, ret_ty);
        }

        p.write(" = ");
        self.format_expr(p, &item.body);
        p.write(";");
        p.newline();
    }

    /// Format an expression.
    /// 格式化表达式。
    fn format_expr(&self, p: &mut Printer, expr: &Expr) {
        match &expr.kind {
            // Literals / 字面量
            ExprKind::Int(n) => p.write(&n.to_string()),
            ExprKind::Float(f) => p.write(&f.to_string()),
            ExprKind::String(s) => {
                p.write("\"");
                p.write(&escape_string(s));
                p.write("\"");
            }
            ExprKind::Char(c) => {
                p.write("'");
                p.write(&escape_char(*c));
                p.write("'");
            }
            ExprKind::Bool(b) => p.write(if *b { "true" } else { "false" }),
            ExprKind::Unit => p.write("()"),

            // Variable / 变量
            ExprKind::Var(ident) => p.write(&ident.name),

            // Path / 路径
            ExprKind::Path(parts) => {
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        p.write(".");
                    }
                    p.write(&part.name);
                }
            }

            // Record / 记录
            ExprKind::Record(fields) => {
                if fields.is_empty() {
                    p.write("#{}");
                } else {
                    // Check if we should break to multiple lines
                    // 检查是否应该拆分为多行
                    let estimated_len: usize = fields
                        .iter()
                        .map(|f| f.name.name.len() + 4) // " = " + ", "
                        .sum();

                    if p.would_exceed_width(estimated_len + 4) && fields.len() > 1 {
                        // Multi-line format / 多行格式
                        p.writeln("#{");
                        p.indent();
                        for (i, field) in fields.iter().enumerate() {
                            self.format_record_field(p, field);
                            if i < fields.len() - 1 {
                                p.write(",");
                            }
                            p.newline();
                        }
                        p.dedent();
                        p.write("}");
                    } else {
                        // Single line format / 单行格式
                        p.write("#{");
                        p.space();
                        for (i, field) in fields.iter().enumerate() {
                            if i > 0 {
                                p.write(",");
                                p.space();
                            }
                            self.format_record_field(p, field);
                        }
                        p.space();
                        p.write("}");
                    }
                }
            }

            // Record update / 记录更新
            ExprKind::RecordUpdate { base, fields } => {
                p.write("#{ ");
                self.format_expr(p, base);
                p.write(" | ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_record_field(p, field);
                }
                p.write(" }");
            }

            // List / 列表
            ExprKind::List(elements) => {
                if elements.is_empty() {
                    p.write("[]");
                } else if elements.len() > 3 && p.would_exceed_width(elements.len() * 5) {
                    // Multi-line list for many elements
                    // 多元素的多行列表
                    p.writeln("[");
                    p.indent();
                    for (i, elem) in elements.iter().enumerate() {
                        self.format_expr(p, elem);
                        if i < elements.len() - 1 {
                            p.write(",");
                        }
                        p.newline();
                    }
                    p.dedent();
                    p.write("]");
                } else {
                    p.write("[");
                    for (i, elem) in elements.iter().enumerate() {
                        if i > 0 {
                            p.write(",");
                            p.space();
                        }
                        self.format_expr(p, elem);
                    }
                    p.write("]");
                }
            }

            // List comprehension / 列表推导
            ExprKind::ListComp { body, generators } => {
                p.write("[");
                self.format_expr(p, body);
                for generator in generators {
                    p.write(" | ");
                    self.format_generator(p, generator);
                }
                p.write("]");
            }

            // Tuple / 元组
            ExprKind::Tuple(elements) => {
                p.write("(");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_expr(p, elem);
                }
                if elements.len() == 1 {
                    p.write(",");
                }
                p.write(")");
            }

            // Lambda / Lambda 表达式
            ExprKind::Lambda { params, body } => {
                p.write("fn(");
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_lambda_param(p, param);
                }
                p.write(") ");
                self.format_expr(p, body);
            }

            // Call / 调用
            ExprKind::Call { func, args } => {
                self.format_expr(p, func);
                p.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_expr(p, arg);
                }
                p.write(")");
            }

            // Method call / 方法调用
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                self.format_expr(p, receiver);
                p.write(".");
                p.write(&method.name);
                p.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_expr(p, arg);
                }
                p.write(")");
            }

            // Field access / 字段访问
            ExprKind::Field { base, field } => {
                self.format_expr(p, base);
                p.write(".");
                p.write(&field.name);
            }

            // Tuple index / 元组索引
            ExprKind::TupleIndex { base, index } => {
                self.format_expr(p, base);
                p.write(".");
                p.write(&index.to_string());
            }

            // Safe field access / 安全字段访问
            ExprKind::SafeField { base, field } => {
                self.format_expr(p, base);
                p.write("?.");
                p.write(&field.name);
            }

            // Index / 索引
            ExprKind::Index { base, index } => {
                self.format_expr(p, base);
                p.write("[");
                self.format_expr(p, index);
                p.write("]");
            }

            // Binary / 二元运算
            ExprKind::Binary { op, left, right } => {
                self.format_expr(p, left);
                p.write(" ");
                p.write(self.binop_str(*op));
                p.write(" ");
                self.format_expr(p, right);
            }

            // Unary / 一元运算
            ExprKind::Unary { op, operand } => {
                p.write(self.unaryop_str(*op));
                self.format_expr(p, operand);
            }

            // Try (error propagation) / Try（错误传播）
            ExprKind::Try(inner) => {
                self.format_expr(p, inner);
                p.write("?");
            }

            // Coalesce / 空值合并
            ExprKind::Coalesce { value, default } => {
                self.format_expr(p, value);
                p.write(" ?? ");
                self.format_expr(p, default);
            }

            // If / 条件表达式
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                p.write("if ");
                self.format_expr(p, condition);
                p.write(" then ");
                self.format_expr(p, then_branch);
                p.write(" else ");
                self.format_expr(p, else_branch);
            }

            // Match / 模式匹配
            ExprKind::Match { scrutinee, arms } => {
                p.write("match ");
                self.format_expr(p, scrutinee);
                p.write(" {");
                p.newline();
                p.indent();
                for arm in arms {
                    self.format_match_arm(p, arm);
                }
                p.dedent();
                p.write("}");
            }

            // Block / 块
            ExprKind::Block { stmts, expr } => {
                p.write("{");
                if stmts.is_empty() && expr.is_none() {
                    p.write("}");
                } else {
                    p.newline();
                    p.indent();
                    for stmt in stmts {
                        self.format_stmt(p, stmt);
                    }
                    if let Some(e) = expr {
                        self.format_expr(p, e);
                        p.newline();
                    }
                    p.dedent();
                    p.write("}");
                }
            }

            // Let expression / Let 表达式
            ExprKind::Let {
                pattern,
                ty,
                value,
                body,
            } => {
                p.write("let ");
                self.format_pattern(p, pattern);
                if let Some(t) = ty {
                    p.write(": ");
                    self.format_type(p, t);
                }
                p.write(" = ");
                self.format_expr(p, value);
                p.write("; ");
                self.format_expr(p, body);
            }

            // Lazy / 惰性求值
            ExprKind::Lazy(inner) => {
                p.write("lazy ");
                self.format_expr(p, inner);
            }

            // Interpolated string / 插值字符串
            ExprKind::Interpolated(parts) => {
                p.write("`");
                for part in parts {
                    match part {
                        StringPart::Literal(s) => {
                            // Escape backticks and braces in literal parts
                            // 在字面量部分转义反引号和大括号
                            for c in s.chars() {
                                match c {
                                    '`' => p.write("\\`"),
                                    '{' => p.write("\\{"),
                                    '}' => p.write("\\}"),
                                    '\n' => p.write("\\n"),
                                    '\r' => p.write("\\r"),
                                    '\t' => p.write("\\t"),
                                    _ => p.write(&c.to_string()),
                                }
                            }
                        }
                        StringPart::Expr(e) => {
                            p.write("{");
                            self.format_expr(p, e);
                            p.write("}");
                        }
                    }
                }
                p.write("`");
            }

            // Path literal (./foo, ../bar, /absolute/path)
            // 路径字面量（./foo、../bar、/absolute/path）
            ExprKind::PathLit(path) => {
                p.write(path);
            }
        }
    }

    /// Format a record field in an expression.
    /// 格式化表达式中的记录字段。
    fn format_record_field(&self, p: &mut Printer, field: &RecordField) {
        p.write(&field.name.name);
        if let Some(ref value) = field.value {
            p.write(" = ");
            self.format_expr(p, value);
        }
    }

    /// Format a generator in list comprehension.
    /// 格式化列表推导中的生成器。
    fn format_generator(&self, p: &mut Printer, generator: &Generator) {
        self.format_pattern(p, &generator.pattern);
        p.write(" <- ");
        self.format_expr(p, &generator.iter);
        if let Some(cond) = &generator.condition {
            p.write(", ");
            self.format_expr(p, cond);
        }
    }

    /// Format a lambda parameter.
    /// 格式化 lambda 参数。
    fn format_lambda_param(&self, p: &mut Printer, param: &LambdaParam) {
        self.format_pattern(p, &param.pattern);
        if let Some(ref ty) = param.ty {
            p.write(": ");
            self.format_type(p, ty);
        }
    }

    /// Format a match arm.
    /// 格式化匹配分支。
    fn format_match_arm(&self, p: &mut Printer, arm: &MatchArm) {
        self.format_pattern(p, &arm.pattern);
        if let Some(ref guard) = arm.guard {
            p.write(" if ");
            self.format_expr(p, guard);
        }
        p.write(" => ");
        self.format_expr(p, &arm.body);
        p.write(",");
        p.newline();
    }

    /// Format a statement.
    /// 格式化语句。
    fn format_stmt(&self, p: &mut Printer, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, ty, value } => {
                p.write("let ");
                self.format_pattern(p, pattern);
                if let Some(t) = ty {
                    p.write(": ");
                    self.format_type(p, t);
                }
                p.write(" = ");
                self.format_expr(p, value);
                p.write(";");
                p.newline();
            }
            StmtKind::Expr(e) => {
                self.format_expr(p, e);
                p.write(";");
                p.newline();
            }
        }
    }

    /// Format a pattern.
    /// 格式化模式。
    fn format_pattern(&self, p: &mut Printer, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Wildcard => p.write("_"),
            PatternKind::Var(ident) => p.write(&ident.name),
            PatternKind::Literal(lit) => self.format_literal_pattern(p, lit),
            PatternKind::Tuple(patterns) => {
                p.write("(");
                for (i, pat) in patterns.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_pattern(p, pat);
                }
                p.write(")");
            }
            PatternKind::List(patterns) => {
                p.write("[");
                for (i, pat) in patterns.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_pattern(p, pat);
                }
                p.write("]");
            }
            PatternKind::ListRest { init, rest, tail } => {
                p.write("[");
                for (i, pat) in init.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_pattern(p, pat);
                }
                if !init.is_empty() && (rest.is_some() || !tail.is_empty()) {
                    p.write(", ");
                }
                if let Some(r) = rest {
                    p.write("..");
                    self.format_pattern(p, r);
                } else {
                    p.write("..");
                }
                for pat in tail.iter() {
                    p.write(", ");
                    self.format_pattern(p, pat);
                }
                p.write("]");
            }
            PatternKind::Record { fields, rest } => {
                p.write("#{ ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_record_pattern_field(p, field);
                }
                if *rest {
                    if !fields.is_empty() {
                        p.write(", ");
                    }
                    p.write("..");
                }
                p.write(" }");
            }
            PatternKind::Constructor { path, args } => {
                for (i, part) in path.iter().enumerate() {
                    if i > 0 {
                        p.write(".");
                    }
                    p.write(&part.name);
                }
                if !args.is_empty() {
                    p.write("(");
                    for (i, pat) in args.iter().enumerate() {
                        if i > 0 {
                            p.write(", ");
                        }
                        self.format_pattern(p, pat);
                    }
                    p.write(")");
                }
            }
            PatternKind::Or(patterns) => {
                for (i, pat) in patterns.iter().enumerate() {
                    if i > 0 {
                        p.write(" | ");
                    }
                    self.format_pattern(p, pat);
                }
            }
            PatternKind::Binding { name, pattern } => {
                p.write(&name.name);
                p.write(" @ ");
                self.format_pattern(p, pattern);
            }
        }
    }

    /// Format a literal pattern.
    /// 格式化字面量模式。
    fn format_literal_pattern(&self, p: &mut Printer, lit: &LiteralPattern) {
        match lit {
            LiteralPattern::Int(n) => p.write(&n.to_string()),
            LiteralPattern::Float(f) => p.write(&f.to_string()),
            LiteralPattern::String(s) => {
                p.write("\"");
                p.write(&escape_string(s));
                p.write("\"");
            }
            LiteralPattern::Char(c) => {
                p.write("'");
                p.write(&escape_char(*c));
                p.write("'");
            }
            LiteralPattern::Bool(b) => p.write(if *b { "true" } else { "false" }),
        }
    }

    /// Format a record pattern field.
    /// 格式化记录模式字段。
    fn format_record_pattern_field(&self, p: &mut Printer, field: &RecordPatternField) {
        p.write(&field.name.name);
        if let Some(ref pat) = field.pattern {
            p.write(": ");
            self.format_pattern(p, pat);
        }
    }

    /// Format a type.
    /// 格式化类型。
    fn format_type(&self, p: &mut Printer, ty: &Type) {
        match &ty.kind {
            TypeKind::Named { path, args } => {
                for (i, part) in path.iter().enumerate() {
                    if i > 0 {
                        p.write(".");
                    }
                    p.write(&part.name);
                }
                if !args.is_empty() {
                    p.write("<");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            p.write(", ");
                        }
                        self.format_type(p, arg);
                    }
                    p.write(">");
                }
            }
            TypeKind::Function { params, result } => {
                if params.len() == 1 {
                    self.format_type(p, &params[0]);
                } else {
                    p.write("(");
                    for (i, param) in params.iter().enumerate() {
                        if i > 0 {
                            p.write(", ");
                        }
                        self.format_type(p, param);
                    }
                    p.write(")");
                }
                p.write(" -> ");
                self.format_type(p, result);
            }
            TypeKind::Tuple(elements) => {
                p.write("(");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_type(p, elem);
                }
                p.write(")");
            }
            TypeKind::Record(fields) => {
                p.write("#{ ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        p.write(", ");
                    }
                    self.format_record_type_field(p, field);
                }
                p.write(" }");
            }
            TypeKind::Unit => p.write("()"),
            TypeKind::Infer => p.write("_"),
        }
    }

    /// Format a record type field.
    /// 格式化记录类型字段。
    fn format_record_type_field(&self, p: &mut Printer, field: &RecordTypeField) {
        p.write(&field.name.name);
        p.write(": ");
        self.format_type(p, &field.ty);
    }

    /// Get the string representation of a binary operator.
    /// 获取二元运算符的字符串表示。
    fn binop_str(&self, op: BinOp) -> &'static str {
        match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Pow => "^",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::Concat => "++",
            BinOp::Merge => "//",
            BinOp::Pipe => "|>",
        }
    }

    /// Get the string representation of a unary operator.
    /// 获取一元运算符的字符串表示。
    fn unaryop_str(&self, op: UnaryOp) -> &'static str {
        match op {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        }
    }
}

/// Escape special characters in a string.
/// 转义字符串中的特殊字符。
fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

/// Escape a character.
/// 转义字符。
fn escape_char(c: char) -> String {
    match c {
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        _ => c.to_string(),
    }
}
