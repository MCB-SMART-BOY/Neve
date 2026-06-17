//! Integration tests for neve-fmt crate.

use neve_fmt::printer::Printer;
use neve_fmt::{FormatConfig, Formatter, check, format};
use neve_lexer::Lexer;
use neve_parser::Parser;

fn format_code(source: &str) -> String {
    let lexer = Lexer::new(source);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse_file();

    let formatter = Formatter::new(FormatConfig::default());
    formatter.format(&ast).expect("formatter should succeed")
}

// FormatConfig tests

#[test]
fn test_default_config() {
    let config = FormatConfig::default();
    assert_eq!(config.indent_width, 2);
    assert_eq!(config.max_width, 100);
    assert!(!config.use_tabs);
}

#[test]
fn test_indent_str() {
    let config = FormatConfig::new().indent_width(4);
    assert_eq!(config.indent_str(), "    ");

    let config = FormatConfig::new().use_tabs(true);
    assert_eq!(config.indent_str(), "\t");
}

#[test]
fn test_custom_config() {
    let config = FormatConfig::new()
        .indent_width(4)
        .max_width(80)
        .use_tabs(false);

    assert_eq!(config.indent_width, 4);
    assert_eq!(config.max_width, 80);
}

// Format tests

#[test]
fn test_format_simple() {
    let source = "let x=1;";
    let formatted = format(source).unwrap();
    assert!(formatted.contains("x = 1"));
}

#[test]
fn test_check() {
    let source = "let x = 1;\n";
    let result = check(source);
    assert!(result.is_ok());
}

#[test]
fn test_format_let() {
    let formatted = format_code("let x = 1;");
    assert!(formatted.contains("x = 1"));
}

#[test]
fn test_format_function() {
    let formatted = format_code("fn add(a: Int, b: Int) -> Int = a + b;");
    assert!(formatted.contains("add(a:"));
}

#[test]
fn test_format_record() {
    let formatted = format_code("r = { a = 1, b = 2, }");
    assert!(formatted.contains("{ a = 1, b = 2 }"));
}

#[test]
fn test_format_list() {
    let formatted = format_code("let xs = [1, 2, 3];");
    assert!(formatted.contains("[1, 2, 3]"));
}

#[test]
fn test_format_if() {
    let formatted = format_code("let x = if true -> 1 else 2;");
    assert!(formatted.contains("if true ->"));
}

#[test]
fn test_format_trait_assoc_type() {
    let formatted = format_code("trait Iterator { type Item: Show; fn next(self) -> Item; };");
    assert!(formatted.contains("type Item: Show;"));
}

#[test]
fn test_format_impl_assoc_type() {
    let formatted = format_code("impl Iterator for Foo { type Item = Int; };");
    assert!(formatted.contains("type Item = Int;"));
}

#[test]
fn test_format_parse_error() {
    let result = format("let x =");
    assert!(result.is_err());
}

#[test]
fn test_format_error_diagnostics() {
    let err = format("let x =").unwrap_err();
    let diags = err.diagnostics();
    assert!(diags.is_some());
    assert!(!diags.unwrap().is_empty());
}

// Idempotence tests — formatting already-formatted code must be stable

fn assert_idempotent(source: &str) {
    let first = format(source).expect("first format should succeed");
    let second = format(&first).expect("second format should succeed");
    assert_eq!(
        first, second,
        "formatter is not idempotent:\nsource: {source}\nfirst:  {first}\nsecond: {second}"
    );
}

#[test]
fn test_idempotent_simple_let() {
    assert_idempotent("let x = 1;\n");
}

#[test]
fn test_idempotent_function_def() {
    assert_idempotent("fn add(a: Int, b: Int) -> Int = a + b;\n");
}

#[test]
fn test_idempotent_record() {
    assert_idempotent("let r = #{ a = 1, b = 2 };\n");
}

#[test]
fn test_idempotent_list() {
    assert_idempotent("let xs = [1, 2, 3];\n");
}

#[test]
fn test_idempotent_if_else() {
    assert_idempotent("let x = if true -> 1 else 2;\n");
}

#[test]
fn test_idempotent_match() {
    assert_idempotent("let x = match 1 { 0 -> 0, _ -> 1 };\n");
}

#[test]
fn test_idempotent_block() {
    assert_idempotent("let x = { let y = 1; y + 2 };\n");
}

#[test]
fn test_idempotent_pipe() {
    assert_idempotent("x = 1 |> |x| x + 1\n");
}

#[test]
fn test_idempotent_lambda() {
    assert_idempotent("let f = fn(x: Int) x + 1;\n");
}

#[test]
fn test_idempotent_enum() {
    assert_idempotent("enum Option { Some(Int), None };\n");
}

#[test]
fn test_idempotent_struct() {
    assert_idempotent("struct Point { x: Int, y: Int };\n");
}

#[test]
fn test_idempotent_trait() {
    assert_idempotent("trait Show { fn show(self) -> String; };\n");
}

#[test]
fn test_idempotent_import() {
    assert_idempotent("use std.io = io;\n");
}

#[test]
fn test_idempotent_effect_fn() {
    assert_idempotent("fn run() -> Unit = { () };\n");
}

#[test]
fn test_idempotent_type_alias() {
    assert_idempotent("type Name = String;\n");
}

// Printer tests

#[test]
fn test_printer_basic() {
    let config = FormatConfig::default();
    let mut printer = Printer::new(config);

    printer.write("hello");
    printer.space();
    printer.write("world");

    let output = printer.finish();
    assert_eq!(output, "hello world\n");
}

#[test]
fn test_printer_indent() {
    let config = FormatConfig::new().indent_width(2);
    let mut printer = Printer::new(config);

    printer.writeln("let x =");
    printer.indent();
    printer.writeln("1");
    printer.dedent();

    let output = printer.finish();
    assert!(output.contains("  1"));
}

#[test]
fn test_printer_newline() {
    let config = FormatConfig::default();
    let mut printer = Printer::new(config);

    printer.write("a");
    printer.newline();
    printer.write("b");

    let output = printer.finish();
    assert!(output.contains("a\nb"));
}
