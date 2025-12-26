//! AST formatter.

use crate::config::FormatConfig;
use crate::printer::Printer;
use neve_syntax::{
    Expr, ExprKind, BinOp, UnaryOp, RecordField, MatchArm, LambdaParam,
    Stmt, StmtKind, Generator,
    Pattern, PatternKind, LiteralPattern, RecordPatternField,
    Type, TypeKind, RecordTypeField,
    SourceFile, Item, ItemKind, LetDef, FnDef, TypeAlias, StructDef,
    EnumDef, TraitDef, ImplDef, ImportDef, ImportItems,
    Param, GenericParam, FieldDef, VariantKind,
    TraitItem, ImplItem,
};

/// Code formatter.
pub struct Formatter {
    config: FormatConfig,
}

impl Formatter {
    /// Create a new formatter.
    pub fn new(config: FormatConfig) -> Self {
        Self { config }
    }

    /// Format a source file.
    pub fn format(&self, file: &SourceFile) -> String {
        let mut printer = Printer::new(self.config.clone());
        
        for (i, item) in file.items.iter().enumerate() {
            if i > 0 {
                // Add extra blank line between top-level items if configured
                if printer.config().blank_lines_between_items {
                    printer.newline();
                }
                printer.newline();
            }
            self.format_item(&mut printer, item);
        }
        
        // Ensure we're at indent level 0 at end of file
        debug_assert_eq!(printer.current_indent(), 0, "unbalanced indentation");
        
        printer.finish()
    }

    /// Format an item.
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
    fn format_let(&self, p: &mut Printer, def: &LetDef) {
        if def.is_pub {
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
    fn format_fn(&self, p: &mut Printer, def: &FnDef) {
        if def.is_pub {
            p.write("pub ");
        }
        p.write("fn ");
        p.write(&def.name.name);
        
        // Generics
        self.format_generics(p, &def.generics);
        
        // Parameters
        p.write("(");
        for (i, param) in def.params.iter().enumerate() {
            if i > 0 {
                p.write(", ");
            }
            self.format_param(p, param);
        }
        p.write(")");
        
        // Return type
        if let Some(ref ret_ty) = def.return_type {
            p.write(" -> ");
            self.format_type(p, ret_ty);
        }
        
        // Body
        p.write(" = ");
        self.format_expr(p, &def.body);
        p.write(";");
        p.newline();
    }

    /// Format a type alias.
    fn format_type_alias(&self, p: &mut Printer, def: &TypeAlias) {
        if def.is_pub {
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
    fn format_struct(&self, p: &mut Printer, def: &StructDef) {
        if def.is_pub {
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
    fn format_enum(&self, p: &mut Printer, def: &EnumDef) {
        if def.is_pub {
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
    fn format_trait(&self, p: &mut Printer, def: &TraitDef) {
        if def.is_pub {
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
    fn format_param(&self, p: &mut Printer, param: &Param) {
        if param.is_lazy {
            p.write("lazy ");
        }
        self.format_pattern(p, &param.pattern);
        p.write(": ");
        self.format_type(p, &param.ty);
    }

    /// Format a field definition.
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
    fn format_expr(&self, p: &mut Printer, expr: &Expr) {
        match &expr.kind {
            // Literals
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

            // Variable
            ExprKind::Var(ident) => p.write(&ident.name),

            // Path
            ExprKind::Path(parts) => {
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        p.write(".");
                    }
                    p.write(&part.name);
                }
            }

            // Record
            ExprKind::Record(fields) => {
                if fields.is_empty() {
                    p.write("#{}");
                } else {
                    // Check if we should break to multiple lines
                    let estimated_len: usize = fields.iter()
                        .map(|f| f.name.name.len() + 4) // " = " + ", "
                        .sum();
                    
                    if p.would_exceed_width(estimated_len + 4) && fields.len() > 1 {
                        // Multi-line format
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
                        // Single line format
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

            // Record update
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

            // List
            ExprKind::List(elements) => {
                if elements.is_empty() {
                    p.write("[]");
                } else if elements.len() > 3 && p.would_exceed_width(elements.len() * 5) {
                    // Multi-line list for many elements
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

            // List comprehension
            ExprKind::ListComp { body, generators } => {
                p.write("[");
                self.format_expr(p, body);
                for generator in generators {
                    p.write(" | ");
                    self.format_generator(p, generator);
                }
                p.write("]");
            }

            // Tuple
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

            // Lambda
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

            // Call
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

            // Method call
            ExprKind::MethodCall { receiver, method, args } => {
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

            // Field access
            ExprKind::Field { base, field } => {
                self.format_expr(p, base);
                p.write(".");
                p.write(&field.name);
            }

            // Tuple index
            ExprKind::TupleIndex { base, index } => {
                self.format_expr(p, base);
                p.write(".");
                p.write(&index.to_string());
            }

            // Safe field access
            ExprKind::SafeField { base, field } => {
                self.format_expr(p, base);
                p.write("?.");
                p.write(&field.name);
            }

            // Index
            ExprKind::Index { base, index } => {
                self.format_expr(p, base);
                p.write("[");
                self.format_expr(p, index);
                p.write("]");
            }

            // Binary
            ExprKind::Binary { op, left, right } => {
                self.format_expr(p, left);
                p.write(" ");
                p.write(self.binop_str(*op));
                p.write(" ");
                self.format_expr(p, right);
            }

            // Unary
            ExprKind::Unary { op, operand } => {
                p.write(self.unaryop_str(*op));
                self.format_expr(p, operand);
            }

            // Try (error propagation)
            ExprKind::Try(inner) => {
                self.format_expr(p, inner);
                p.write("?");
            }

            // Coalesce
            ExprKind::Coalesce { value, default } => {
                self.format_expr(p, value);
                p.write(" ?? ");
                self.format_expr(p, default);
            }

            // If
            ExprKind::If { condition, then_branch, else_branch } => {
                p.write("if ");
                self.format_expr(p, condition);
                p.write(" then ");
                self.format_expr(p, then_branch);
                p.write(" else ");
                self.format_expr(p, else_branch);
            }

            // Match
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

            // Block
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

            // Let expression
            ExprKind::Let { pattern, ty, value, body } => {
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

            // Lazy
            ExprKind::Lazy(inner) => {
                p.write("lazy ");
                self.format_expr(p, inner);
            }
        }
    }

    /// Format a record field in an expression.
    fn format_record_field(&self, p: &mut Printer, field: &RecordField) {
        p.write(&field.name.name);
        if let Some(ref value) = field.value {
            p.write(" = ");
            self.format_expr(p, value);
        }
    }

    /// Format a generator in list comprehension.
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
    fn format_lambda_param(&self, p: &mut Printer, param: &LambdaParam) {
        self.format_pattern(p, &param.pattern);
        if let Some(ref ty) = param.ty {
            p.write(": ");
            self.format_type(p, ty);
        }
    }

    /// Format a match arm.
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
    fn format_record_pattern_field(&self, p: &mut Printer, field: &RecordPatternField) {
        p.write(&field.name.name);
        if let Some(ref pat) = field.pattern {
            p.write(": ");
            self.format_pattern(p, pat);
        }
    }

    /// Format a type.
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
    fn format_record_type_field(&self, p: &mut Printer, field: &RecordTypeField) {
        p.write(&field.name.name);
        p.write(": ");
        self.format_type(p, &field.ty);
    }

    /// Get the string representation of a binary operator.
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
    fn unaryop_str(&self, op: UnaryOp) -> &'static str {
        match op {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        }
    }
}

/// Escape special characters in a string.
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

#[cfg(test)]
mod tests {
    use super::*;
    use neve_lexer::Lexer;
    use neve_parser::Parser;

    fn format_code(source: &str) -> String {
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_file();
        
        let formatter = Formatter::new(FormatConfig::default());
        formatter.format(&ast)
    }

    #[test]
    fn test_format_let() {
        let formatted = format_code("let x = 1;");
        assert!(formatted.contains("let x = 1;"));
    }

    #[test]
    fn test_format_function() {
        let formatted = format_code("fn add(a: Int, b: Int) -> Int = a + b;");
        assert!(formatted.contains("fn add"));
    }

    #[test]
    fn test_format_record() {
        let formatted = format_code("let r = #{ a = 1, b = 2 };");
        assert!(formatted.contains("#{ a = 1, b = 2 }"));
    }
}
