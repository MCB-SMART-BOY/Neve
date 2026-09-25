//! Integration tests for n3v3-fmt crate.

use n3v3_diagnostic::Severity;
use n3v3_fmt::printer::Printer;
use n3v3_fmt::{FormatConfig, Formatter, check, format};
use n3v3_frontend::analyze_source;
use n3v3_lexer::Lexer;
use n3v3_parser::Parser;

/// Format `source`, then require the result to survive the canonical pipeline
/// (`Lexer -> Parser -> HIR -> Typeck`). Formatted output that fails to parse, or
/// that only parses into something else, is exactly the regression these tests
/// guard: a parser-only check would miss the second kind.
/// 格式化 `source` 后要求结果能通过规范流水线（`Lexer -> Parser -> HIR -> Typeck`）。
/// 打印结果无法解析、或解析成别的东西，正是这些测试要防的回归；只做解析检查会漏掉
/// 第二种。
fn assert_formatted_analyzes_clean(source: &str) -> String {
    let formatted = format(source).expect("format should succeed");
    let analysis = analyze_source(&formatted);
    let errors: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| format!("{:?}: {}", diagnostic.kind, diagnostic.message))
        .collect();
    assert!(
        errors.is_empty(),
        "formatted output must analyze without errors:\nsource: {source}\nformatted: {formatted}\nerrors: {errors:?}"
    );
    formatted
}

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

#[test]
fn test_format_preserves_comment_text() {
    let source = "-- first\nlet x = 1; -- inline\n-- -- block\n-- --\n";
    let formatted = format(source).expect("format with comments should succeed");

    assert!(formatted.contains("-- first"));
    assert!(formatted.contains("-- inline"));
    assert!(formatted.contains("-- -- block\n-- --"));
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

// List-rest patterns: `..` must keep its `, ` separator, otherwise the output
// does not parse (`[x..]`). Regression for a printer that gated the separator
// on rest/tail instead of on `init`.
// 列表剩余模式：`..` 前的 `, ` 不能丢，否则输出无法解析（`[x..]`）。
#[test]
fn test_format_list_rest_pattern_keeps_separator() {
    let formatted = format("let f = |xs| { match xs { [x, ..] -> 1, _ -> 0 } };\n")
        .expect("format should succeed");
    assert!(
        formatted.contains("[x, ..]"),
        "list-rest pattern lost its separator:\n{formatted}"
    );
}

#[test]
fn test_idempotent_list_rest_patterns() {
    assert_idempotent(
        "let f = |xs| { match xs { [x, ..] -> 1, [x, ..rest] -> 2, [.., last] -> 3, [..] -> 4, _ -> 0 } };\n",
    );
}

// Zero-parameter lambdas: `||` is the logical-or token, so the printer must use
// the `fn()` spelling or the output cannot be re-parsed.
// 零参数 lambda：`||` 是逻辑或 token，打印时必须用 `fn()` 形式，否则无法重新解析。
#[test]
fn test_format_zero_param_lambda_is_parseable() {
    let formatted = format("let f = fn() { 1 };\n").expect("format should succeed");
    assert!(
        formatted.contains("fn()"),
        "zero-param lambda must not be printed as `||`:\n{formatted}"
    );
    assert_idempotent("let f = fn() { 1 };\n");
}

#[test]
fn test_format_zero_param_lambda_with_return_type_is_parseable() {
    assert_idempotent("let f = fn() -> Int { 1 };\n");
}

// A bare `{...}` after lambda parameters is parsed as a block, not a record, so a
// record body must be printed inside parentheses to stay parseable.
// lambda 参数后的裸 `{...}` 会被解析为块而非记录，因此记录体必须打印为带括号形式。
#[test]
fn test_format_lambda_record_body_is_parenthesized() {
    let source = "let f = |x| ({\n  a = 1,\n  b = 2\n});\n";
    let formatted = format(source).expect("format should succeed");
    assert!(
        formatted.contains("({"),
        "record lambda body must be parenthesized:\n{formatted}"
    );
    assert_idempotent(source);
}

#[test]
fn test_format_lambda_record_update_body_is_parenthesized() {
    // `RecordUpdate` also prints as `{ base | fields }`, so it needs the same
    // parentheses: without them the body parses as a block and the field list
    // becomes an or-pattern (E0200), i.e. `n3v3 fmt --write` corrupted the file.
    // `RecordUpdate` 同样打印为 `{ base | fields }`，因此需要同样的括号：否则函数体会
    // 按块解析，字段列表被当作 or 模式（E0200），也就是 `n3v3 fmt --write` 会改坏文件。
    let source = "let f = |r| ({ r | a = 1 });\n";
    let formatted = assert_formatted_analyzes_clean(source);
    assert!(
        formatted.contains("({"),
        "record-update lambda body must be parenthesized:\n{formatted}"
    );
    assert_idempotent(source);
}

#[test]
fn test_format_single_element_tuple_pattern_keeps_comma() {
    // Without the trailing comma a 1-tuple pattern prints as `(x)` and binds the
    // whole tuple instead of destructuring it - a silent meaning change.
    // 缺少尾逗号时一元组模式会打印成 `(x)`，绑定整个元组而不是解构它——静默改变含义。
    let source = "let t = (1,);\nlet (x,) = t;\n";
    let formatted = assert_formatted_analyzes_clean(source);
    assert!(
        formatted.contains("(x,)"),
        "1-tuple pattern must keep its trailing comma:\n{formatted}"
    );
    assert_idempotent(source);
}

#[test]
fn test_format_top_level_non_variable_pattern_keeps_let() {
    // The top-level `let` is only optional for `x = ...`/`_ = ...`; `(a, b) = t`
    // and `[a, b] = xs` do not parse, so dropping the keyword printed broken files.
    // 顶层 `let` 只对 `x = ...`/`_ = ...` 可省略；`(a, b) = t`、`[a, b] = xs` 无法解析，
    // 省略关键字会打印出坏文件。
    for source in [
        "let t = (1, 2);\nlet (x, y) = t;\n",
        "let xs = [1, 2];\nlet [a, b] = xs;\n",
        "let t = (1, 2);\nlet (_, y) = t;\n",
    ] {
        let formatted = assert_formatted_analyzes_clean(source);
        assert!(
            formatted.contains("let "),
            "non-variable top-level pattern must keep `let`:\nsource: {source}\nformatted: {formatted}"
        );
        assert_idempotent(source);
    }
}

#[test]
fn test_format_top_level_variable_pattern_omits_let() {
    // The common `x = 1` form stays in its canonical (keyword-less) shape.
    // 常见的 `x = 1` 形式保持无关键字的规范形态。
    let formatted = assert_formatted_analyzes_clean("let x = 1;\nlet _ = 2;\n");
    assert_eq!(
        formatted, "x = 1\n\n_ = 2\n",
        "variable and wildcard patterns must keep the bare form"
    );
}

#[test]
fn test_format_lambda_block_body_stays_unparenthesized() {
    let source = "let f = |x| {\n  let y = x;\n  y\n};\n";
    let formatted = format(source).expect("format should succeed");
    assert!(
        !formatted.contains("({"),
        "block bodies must not be parenthesized:\n{formatted}"
    );
    assert_idempotent(source);
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
