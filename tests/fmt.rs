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
    formatter.format(&ast)
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
    let source = "let x=1";
    let formatted = format(source).unwrap();
    assert!(formatted.contains("let x = 1"));
}

#[test]
fn test_check() {
    let source = "let x = 1\n";
    let result = check(source);
    assert!(result.is_ok());
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

#[test]
fn test_format_list() {
    let formatted = format_code("let xs = [1, 2, 3];");
    assert!(formatted.contains("[1, 2, 3]"));
}

#[test]
fn test_format_if() {
    let formatted = format_code("let x = if true then 1 else 2;");
    assert!(formatted.contains("if true then"));
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
