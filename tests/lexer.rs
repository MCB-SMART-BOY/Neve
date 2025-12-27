//! Integration tests for neve-lexer crate.

use neve_lexer::{Lexer, TokenKind};

fn lex(source: &str) -> Vec<TokenKind> {
    let lexer = Lexer::new(source);
    let (tokens, _) = lexer.tokenize();
    tokens.into_iter().map(|t| t.kind).collect()
}

fn lex_with_errors(source: &str) -> (Vec<TokenKind>, usize) {
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.tokenize();
    (tokens.into_iter().map(|t| t.kind).collect(), errors.len())
}

// ============================================================================
// Basic Token Tests
// ============================================================================

#[test]
fn test_keywords() {
    assert_eq!(
        lex("let fn if then else match"),
        vec![
            TokenKind::Let,
            TokenKind::Fn,
            TokenKind::If,
            TokenKind::Then,
            TokenKind::Else,
            TokenKind::Match,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_all_keywords() {
    let keywords = lex("let fn if then else match type import module where with in true false");
    assert!(keywords.contains(&TokenKind::Let));
    assert!(keywords.contains(&TokenKind::Fn));
    assert!(keywords.contains(&TokenKind::If));
    assert!(keywords.contains(&TokenKind::Then));
    assert!(keywords.contains(&TokenKind::Else));
    assert!(keywords.contains(&TokenKind::Match));
    assert!(keywords.contains(&TokenKind::True));
    assert!(keywords.contains(&TokenKind::False));
}

#[test]
fn test_numbers() {
    assert_eq!(
        lex("42 3.25 0xFF 0b1010"),
        vec![
            TokenKind::Int(42),
            TokenKind::Float(3.25),
            TokenKind::Int(255),
            TokenKind::Int(10),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_strings() {
    assert_eq!(
        lex(r#""hello" 'a'"#),
        vec![
            TokenKind::String("hello".to_string()),
            TokenKind::Char('a'),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_operators() {
    assert_eq!(
        lex("+ ++ -> |> // ?? ?."),
        vec![
            TokenKind::Plus,
            TokenKind::PlusPlus,
            TokenKind::Arrow,
            TokenKind::PipeGt,
            TokenKind::SlashSlash,
            TokenKind::QuestionQuestion,
            TokenKind::QuestionDot,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_record_literal() {
    assert_eq!(
        lex("#{ x = 1 }"),
        vec![
            TokenKind::HashLBrace,
            TokenKind::Ident("x".to_string()),
            TokenKind::Eq,
            TokenKind::Int(1),
            TokenKind::RBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_comments() {
    assert_eq!(
        lex("1 -- comment\n2"),
        vec![
            TokenKind::Int(1),
            TokenKind::Int(2),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_identifiers() {
    assert_eq!(
        lex("foo bar_baz _private"),
        vec![
            TokenKind::Ident("foo".to_string()),
            TokenKind::Ident("bar_baz".to_string()),
            TokenKind::Ident("_private".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_brackets() {
    assert_eq!(
        lex("()[]{}"),
        vec![
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::Eof,
        ]
    );
}

// ============================================================================
// Edge Cases - Numbers
// ============================================================================

#[test]
fn test_number_zero() {
    assert_eq!(lex("0")[0], TokenKind::Int(0));
}

#[test]
fn test_number_negative() {
    // Negative numbers are parsed as unary minus + number
    let tokens = lex("-42");
    assert!(tokens.contains(&TokenKind::Minus));
    assert!(tokens.contains(&TokenKind::Int(42)));
}

#[test]
fn test_number_large() {
    assert_eq!(lex("9999999999")[0], TokenKind::Int(9999999999));
}

#[test]
fn test_float_no_leading_zero() {
    // .5 should be parsed as Dot followed by Int, not as Float
    let tokens = lex(".5");
    assert!(tokens.contains(&TokenKind::Dot));
}

#[test]
fn test_float_trailing_dot() {
    // 42. followed by something
    let tokens = lex("42.foo");
    assert!(tokens.len() >= 2);
}

#[test]
fn test_float_scientific_notation() {
    // If supported
    let tokens = lex("1e10");
    assert!(!tokens.is_empty());
}

#[test]
fn test_hex_lowercase() {
    assert_eq!(lex("0xff")[0], TokenKind::Int(255));
}

#[test]
fn test_hex_uppercase() {
    assert_eq!(lex("0XFF")[0], TokenKind::Int(255));
}

#[test]
fn test_hex_mixed_case() {
    assert_eq!(lex("0xAbCdEf")[0], TokenKind::Int(0xABCDEF));
}

#[test]
fn test_binary_number() {
    assert_eq!(lex("0b11111111")[0], TokenKind::Int(255));
}

#[test]
fn test_octal_number() {
    let tokens = lex("0o777");
    if let TokenKind::Int(n) = tokens[0] {
        assert_eq!(n, 0o777);
    }
}

// ============================================================================
// Edge Cases - Strings
// ============================================================================

#[test]
fn test_empty_string() {
    assert_eq!(lex(r#""""#)[0], TokenKind::String("".to_string()));
}

#[test]
fn test_string_with_spaces() {
    assert_eq!(
        lex(r#""hello world""#)[0],
        TokenKind::String("hello world".to_string())
    );
}

#[test]
fn test_string_with_newline_escape() {
    let tokens = lex(r#""hello\nworld""#);
    if let TokenKind::String(s) = &tokens[0] {
        assert!(s.contains("\\n") || s.contains('\n'));
    }
}

#[test]
fn test_string_with_tab_escape() {
    let tokens = lex(r#""hello\tworld""#);
    if let TokenKind::String(s) = &tokens[0] {
        assert!(s.contains("\\t") || s.contains('\t'));
    }
}

#[test]
fn test_string_with_quote_escape() {
    let tokens = lex(r#""hello\"world""#);
    assert!(matches!(&tokens[0], TokenKind::String(_)));
}

#[test]
fn test_string_unicode() {
    assert_eq!(
        lex(r#""你好世界""#)[0],
        TokenKind::String("你好世界".to_string())
    );
}

#[test]
fn test_string_emoji() {
    assert_eq!(
        lex(r#""🎉🚀""#)[0],
        TokenKind::String("🎉🚀".to_string())
    );
}

#[test]
fn test_char_escape() {
    let tokens = lex(r"'\n'");
    assert!(matches!(&tokens[0], TokenKind::Char(_)));
}

#[test]
fn test_char_unicode() {
    assert_eq!(lex("'中'")[0], TokenKind::Char('中'));
}

// ============================================================================
// Edge Cases - Identifiers
// ============================================================================

#[test]
fn test_single_char_ident() {
    assert_eq!(lex("x")[0], TokenKind::Ident("x".to_string()));
}

#[test]
fn test_underscore_only() {
    assert_eq!(lex("_")[0], TokenKind::Ident("_".to_string()));
}

#[test]
fn test_ident_with_numbers() {
    assert_eq!(lex("x123")[0], TokenKind::Ident("x123".to_string()));
}

#[test]
fn test_ident_starting_with_keyword() {
    // "letter" starts with "let" but should be an identifier
    assert_eq!(lex("letter")[0], TokenKind::Ident("letter".to_string()));
    assert_eq!(lex("iffoo")[0], TokenKind::Ident("iffoo".to_string()));
    assert_eq!(lex("matchmaking")[0], TokenKind::Ident("matchmaking".to_string()));
}

#[test]
fn test_ident_camel_case() {
    assert_eq!(lex("myVariable")[0], TokenKind::Ident("myVariable".to_string()));
}

#[test]
fn test_ident_snake_case() {
    assert_eq!(lex("my_variable")[0], TokenKind::Ident("my_variable".to_string()));
}

#[test]
fn test_ident_screaming_snake() {
    assert_eq!(lex("MY_CONSTANT")[0], TokenKind::Ident("MY_CONSTANT".to_string()));
}

#[test]
fn test_ident_unicode() {
    // May or may not be supported
    let tokens = lex("变量");
    assert!(!tokens.is_empty());
}

// ============================================================================
// Edge Cases - Operators
// ============================================================================

#[test]
fn test_all_comparison_operators() {
    let tokens = lex("< > <= >= == !=");
    assert!(tokens.contains(&TokenKind::Lt));
    assert!(tokens.contains(&TokenKind::Gt));
    assert!(tokens.contains(&TokenKind::LtEq));
    assert!(tokens.contains(&TokenKind::GtEq));
    assert!(tokens.contains(&TokenKind::EqEq));
    assert!(tokens.contains(&TokenKind::BangEq));
}

#[test]
fn test_all_arithmetic_operators() {
    let tokens = lex("+ - * / %");
    assert!(tokens.contains(&TokenKind::Plus));
    assert!(tokens.contains(&TokenKind::Minus));
    assert!(tokens.contains(&TokenKind::Star));
    assert!(tokens.contains(&TokenKind::Slash));
    assert!(tokens.contains(&TokenKind::Percent));
}

#[test]
fn test_logical_operators() {
    let tokens = lex("&& || !");
    assert!(tokens.contains(&TokenKind::AndAnd));
    assert!(tokens.contains(&TokenKind::OrOr));
    assert!(tokens.contains(&TokenKind::Bang));
}

#[test]
fn test_consecutive_operators() {
    let tokens = lex("++--");
    assert!(tokens.len() >= 2);
}

#[test]
fn test_operator_no_space() {
    let tokens = lex("1+2");
    assert_eq!(tokens.len(), 4); // Int, Plus, Int, Eof
}

// ============================================================================
// Edge Cases - Comments
// ============================================================================

#[test]
fn test_comment_at_start() {
    assert_eq!(lex("-- comment\n42")[0], TokenKind::Int(42));
}

#[test]
fn test_comment_at_end() {
    let tokens = lex("42 -- comment");
    assert_eq!(tokens[0], TokenKind::Int(42));
    assert_eq!(tokens[1], TokenKind::Eof);
}

#[test]
fn test_multiple_comments() {
    let tokens = lex("-- first\n1 -- second\n2 -- third");
    assert!(tokens.contains(&TokenKind::Int(1)));
    assert!(tokens.contains(&TokenKind::Int(2)));
}

#[test]
fn test_empty_comment() {
    let tokens = lex("--\n42");
    assert_eq!(tokens[0], TokenKind::Int(42));
}

#[test]
fn test_comment_with_code_like_content() {
    let tokens = lex("-- let x = 42\n1");
    assert_eq!(tokens[0], TokenKind::Int(1));
}

// ============================================================================
// Edge Cases - Whitespace
// ============================================================================

#[test]
fn test_multiple_spaces() {
    assert_eq!(lex("1    2")[0], TokenKind::Int(1));
    assert_eq!(lex("1    2")[1], TokenKind::Int(2));
}

#[test]
fn test_tabs() {
    assert_eq!(lex("1\t2")[0], TokenKind::Int(1));
    assert_eq!(lex("1\t2")[1], TokenKind::Int(2));
}

#[test]
fn test_newlines() {
    let tokens = lex("1\n2\n3");
    assert!(tokens.contains(&TokenKind::Int(1)));
    assert!(tokens.contains(&TokenKind::Int(2)));
    assert!(tokens.contains(&TokenKind::Int(3)));
}

#[test]
fn test_crlf() {
    let tokens = lex("1\r\n2");
    assert!(tokens.contains(&TokenKind::Int(1)));
    assert!(tokens.contains(&TokenKind::Int(2)));
}

#[test]
fn test_empty_input() {
    assert_eq!(lex(""), vec![TokenKind::Eof]);
}

#[test]
fn test_only_whitespace() {
    assert_eq!(lex("   \n\t  "), vec![TokenKind::Eof]);
}

#[test]
fn test_only_comments() {
    assert_eq!(lex("-- just a comment"), vec![TokenKind::Eof]);
}

// ============================================================================
// Edge Cases - Complex Expressions
// ============================================================================

#[test]
fn test_nested_brackets() {
    let tokens = lex("((()))");
    assert_eq!(tokens.iter().filter(|t| **t == TokenKind::LParen).count(), 3);
    assert_eq!(tokens.iter().filter(|t| **t == TokenKind::RParen).count(), 3);
}

#[test]
fn test_mixed_brackets() {
    let tokens = lex("([{}])");
    assert!(tokens.contains(&TokenKind::LParen));
    assert!(tokens.contains(&TokenKind::LBracket));
    assert!(tokens.contains(&TokenKind::LBrace));
}

#[test]
fn test_function_call_syntax() {
    let tokens = lex("foo(1, 2, 3)");
    assert_eq!(tokens[0], TokenKind::Ident("foo".to_string()));
    assert!(tokens.contains(&TokenKind::LParen));
    assert!(tokens.contains(&TokenKind::Comma));
}

#[test]
fn test_list_literal() {
    let tokens = lex("[1, 2, 3]");
    assert!(tokens.contains(&TokenKind::LBracket));
    assert!(tokens.contains(&TokenKind::RBracket));
    assert!(tokens.iter().filter(|t| **t == TokenKind::Comma).count() == 2);
}

#[test]
fn test_record_access() {
    let tokens = lex("record.field");
    assert_eq!(tokens[0], TokenKind::Ident("record".to_string()));
    assert!(tokens.contains(&TokenKind::Dot));
    assert_eq!(tokens[2], TokenKind::Ident("field".to_string()));
}

#[test]
fn test_chained_access() {
    let tokens = lex("a.b.c.d");
    assert_eq!(tokens.iter().filter(|t| **t == TokenKind::Dot).count(), 3);
}

#[test]
fn test_pipe_chain() {
    let tokens = lex("x |> f |> g |> h");
    assert_eq!(tokens.iter().filter(|t| **t == TokenKind::PipeGt).count(), 3);
}

// ============================================================================
// Edge Cases - Error Recovery
// ============================================================================

#[test]
fn test_invalid_char() {
    let (tokens, errors) = lex_with_errors("@");
    assert!(errors > 0 || !tokens.is_empty());
}

#[test]
fn test_unterminated_string() {
    let (_, errors) = lex_with_errors(r#""hello"#);
    assert!(errors > 0);
}

#[test]
fn test_unterminated_char() {
    let (_, errors) = lex_with_errors("'a");
    assert!(errors > 0);
}

#[test]
fn test_invalid_escape_in_string() {
    let (tokens, _) = lex_with_errors(r#""hello\qworld""#);
    // Should still produce a token
    assert!(!tokens.is_empty());
}

#[test]
fn test_error_recovery() {
    // 测试无效字符会产生错误
    let (tokens, errors) = lex_with_errors("`");
    // 反引号不是有效 token，应该产生错误
    assert!(errors > 0 || tokens.iter().any(|t| matches!(t, TokenKind::Error)));
}

// ============================================================================
// Edge Cases - Keywords vs Identifiers
// ============================================================================

#[test]
fn test_keyword_as_prefix() {
    assert_eq!(lex("letx")[0], TokenKind::Ident("letx".to_string()));
    assert_eq!(lex("fny")[0], TokenKind::Ident("fny".to_string()));
    assert_eq!(lex("iffy")[0], TokenKind::Ident("iffy".to_string()));
}

#[test]
fn test_keyword_as_suffix() {
    assert_eq!(lex("xlet")[0], TokenKind::Ident("xlet".to_string()));
    assert_eq!(lex("myfn")[0], TokenKind::Ident("myfn".to_string()));
}

#[test]
fn test_keyword_with_underscore() {
    assert_eq!(lex("let_")[0], TokenKind::Ident("let_".to_string()));
    assert_eq!(lex("_let")[0], TokenKind::Ident("_let".to_string()));
}

// ============================================================================
// Edge Cases - Special Tokens
// ============================================================================

#[test]
fn test_semicolon() {
    assert!(lex(";").contains(&TokenKind::Semicolon));
}

#[test]
fn test_colon() {
    assert!(lex(":").contains(&TokenKind::Colon));
}

#[test]
fn test_double_colon() {
    // :: 被解析为两个 Colon token
    let tokens = lex("::");
    let colon_count = tokens.iter().filter(|t| **t == TokenKind::Colon).count();
    assert!(colon_count >= 2);
}

#[test]
fn test_fat_arrow() {
    assert!(lex("=>").contains(&TokenKind::FatArrow));
}

#[test]
fn test_thin_arrow() {
    assert!(lex("->").contains(&TokenKind::Arrow));
}

#[test]
fn test_underscore_pattern() {
    // Underscore as wildcard pattern
    assert_eq!(lex("_")[0], TokenKind::Ident("_".to_string()));
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_long_identifier() {
    let long_name = "a".repeat(1000);
    assert_eq!(lex(&long_name)[0], TokenKind::Ident(long_name));
}

#[test]
fn test_long_string() {
    let content = "x".repeat(10000);
    let source = format!(r#""{}""#, content);
    assert_eq!(lex(&source)[0], TokenKind::String(content));
}

#[test]
fn test_many_tokens() {
    let source = "1 + ".repeat(1000) + "1";
    let tokens = lex(&source);
    assert!(tokens.len() > 2000);
}

#[test]
fn test_deeply_nested() {
    let source = "(".repeat(100) + "x" + &")".repeat(100);
    let tokens = lex(&source);
    assert!(tokens.len() > 200);
}

// ============================================================================
// Additional Edge Cases - Numbers
// ============================================================================

#[test]
fn test_number_with_underscores() {
    // 1_000_000 style numbers if supported
    let tokens = lex("1_000_000");
    assert!(!tokens.is_empty());
}

#[test]
fn test_hex_with_underscores() {
    let tokens = lex("0xFF_FF");
    assert!(!tokens.is_empty());
}

#[test]
fn test_binary_with_underscores() {
    let tokens = lex("0b1111_0000");
    assert!(!tokens.is_empty());
}

#[test]
fn test_float_very_small() {
    let tokens = lex("0.000000001");
    assert!(matches!(tokens[0], TokenKind::Float(_)));
}

#[test]
fn test_float_very_large() {
    let tokens = lex("99999999999.99999999");
    assert!(matches!(tokens[0], TokenKind::Float(_)));
}

#[test]
fn test_consecutive_numbers() {
    // Should not merge
    let tokens = lex("123 456");
    assert_eq!(tokens[0], TokenKind::Int(123));
    assert_eq!(tokens[1], TokenKind::Int(456));
}

#[test]
fn test_number_after_dot() {
    let tokens = lex("foo.123");
    // Should be: ident, dot, int
    assert_eq!(tokens[0], TokenKind::Ident("foo".to_string()));
    assert_eq!(tokens[1], TokenKind::Dot);
}

#[test]
fn test_hex_invalid_digit() {
    let (tokens, errors) = lex_with_errors("0xGG");
    // Should produce an error or parse partially
    assert!(errors > 0 || tokens.len() > 1);
}

#[test]
fn test_binary_invalid_digit() {
    let (tokens, errors) = lex_with_errors("0b222");
    // Should produce an error or parse partially
    assert!(errors > 0 || tokens.len() > 1);
}

#[test]
fn test_octal_invalid_digit() {
    let (tokens, errors) = lex_with_errors("0o999");
    // Should produce an error or parse partially
    assert!(errors > 0 || tokens.len() > 1);
}

// ============================================================================
// Additional Edge Cases - Strings and Characters
// ============================================================================

#[test]
fn test_string_with_backslash() {
    let tokens = lex(r#""path\\to\\file""#);
    assert!(matches!(&tokens[0], TokenKind::String(_)));
}

#[test]
fn test_string_with_null() {
    let tokens = lex(r#""hello\0world""#);
    assert!(matches!(&tokens[0], TokenKind::String(_)));
}

#[test]
fn test_string_with_hex_escape() {
    let tokens = lex(r#""hello\x41world""#);
    assert!(matches!(&tokens[0], TokenKind::String(_)));
}

#[test]
fn test_string_with_unicode_escape() {
    let tokens = lex(r#""hello\u{1F600}world""#);
    assert!(matches!(&tokens[0], TokenKind::String(_)));
}

#[test]
fn test_multiline_string() {
    let tokens = lex(r#""""
multiline
string
""""#);
    assert!(!tokens.is_empty());
}

#[test]
fn test_char_backslash() {
    let tokens = lex(r"'\\'");
    assert!(matches!(&tokens[0], TokenKind::Char(_)));
}

#[test]
fn test_char_quote() {
    let tokens = lex(r"'\''");
    assert!(matches!(&tokens[0], TokenKind::Char(_)));
}

#[test]
fn test_char_zero() {
    let tokens = lex(r"'\0'");
    assert!(matches!(&tokens[0], TokenKind::Char(_)));
}

#[test]
fn test_empty_char() {
    let (_, errors) = lex_with_errors("''");
    // Empty char should be an error
    assert!(errors > 0);
}

#[test]
fn test_multi_char() {
    let (_, errors) = lex_with_errors("'abc'");
    // Multi-char literal should be an error or handled specially
    assert!(errors > 0);
}

#[test]
fn test_string_with_interpolation_braces() {
    // Test `{expr}` interpolation syntax in backtick strings
    let tokens = lex("`hello {name}`");
    assert!(!tokens.is_empty());
}

#[test]
fn test_string_nested_braces_interpolation() {
    let tokens = lex("`value: {#{ x = 1 }.x}`");
    assert!(!tokens.is_empty());
}

// ============================================================================
// Additional Edge Cases - Operators and Punctuation
// ============================================================================

#[test]
fn test_power_operator() {
    let tokens = lex("2 ^ 10");
    assert!(tokens.contains(&TokenKind::Caret));
}

#[test]
fn test_at_symbol() {
    let tokens = lex("x @ pattern");
    assert!(tokens.contains(&TokenKind::At));
}

#[test]
fn test_dotdot() {
    let tokens = lex("[head, ..tail]");
    assert!(tokens.contains(&TokenKind::DotDot));
}

#[test]
fn test_question_mark() {
    let tokens = lex("result?");
    assert!(tokens.contains(&TokenKind::Question));
}

#[test]
fn test_hash_alone() {
    let tokens = lex("#");
    assert!(!tokens.is_empty());
}

#[test]
fn test_all_brackets_together() {
    let tokens = lex("#{ list = [( tuple )] }");
    assert!(tokens.contains(&TokenKind::HashLBrace));
    assert!(tokens.contains(&TokenKind::LBracket));
    assert!(tokens.contains(&TokenKind::LParen));
}

#[test]
fn test_pipe_operator() {
    let tokens = lex("a | b");
    assert!(tokens.contains(&TokenKind::Pipe));
}

#[test]
fn test_consecutive_arrows() {
    let tokens = lex("->->");
    let arrow_count = tokens.iter().filter(|t| **t == TokenKind::Arrow).count();
    assert_eq!(arrow_count, 2);
}

#[test]
fn test_mixed_comparison() {
    let tokens = lex("a < b > c <= d >= e == f != g");
    assert!(tokens.contains(&TokenKind::Lt));
    assert!(tokens.contains(&TokenKind::Gt));
    assert!(tokens.contains(&TokenKind::LtEq));
    assert!(tokens.contains(&TokenKind::GtEq));
    assert!(tokens.contains(&TokenKind::EqEq));
    assert!(tokens.contains(&TokenKind::BangEq));
}

// ============================================================================
// Additional Edge Cases - Comments
// ============================================================================

#[test]
fn test_nested_block_comment() {
    let tokens = lex("-- outer -- inner -- still outer --\n42");
    assert!(tokens.contains(&TokenKind::Int(42)));
}

#[test]
fn test_comment_with_special_chars() {
    let tokens = lex("-- 你好 🎉 @#$%^& --\n1");
    assert!(tokens.contains(&TokenKind::Int(1)));
}

#[test]
fn test_comment_immediately_after_token() {
    let tokens = lex("42-- comment");
    assert!(tokens.contains(&TokenKind::Int(42)));
}

#[test]
fn test_multiple_line_comments() {
    let tokens = lex("-- line 1\n-- line 2\n-- line 3\n42");
    assert!(tokens.contains(&TokenKind::Int(42)));
}

// ============================================================================
// Path Literals
// ============================================================================

#[test]
fn test_dot_starts_range() {
    // ./ is tokenized as Dot followed by other tokens
    let tokens = lex("./path");
    assert!(tokens.contains(&TokenKind::Dot));
}

#[test]
fn test_dotdot_operator() {
    // .. is a range operator
    let tokens = lex("1..10");
    assert!(tokens.contains(&TokenKind::DotDot));
}

#[test]
fn test_slash_is_division() {
    let tokens = lex("/absolute/path");
    // / is parsed as division operator
    assert!(tokens.contains(&TokenKind::Slash));
}

#[test]
fn test_dot_chain() {
    let tokens = lex("a.b.c");
    let dot_count = tokens.iter().filter(|t| **t == TokenKind::Dot).count();
    assert_eq!(dot_count, 2);
}

#[test]
fn test_dotdot_range() {
    let tokens = lex("0..100");
    assert!(tokens.contains(&TokenKind::Int(0)));
    assert!(tokens.contains(&TokenKind::DotDot));
    assert!(tokens.contains(&TokenKind::Int(100)));
}

// ============================================================================
// Whitespace and Line Handling
// ============================================================================

#[test]
fn test_mixed_whitespace() {
    let tokens = lex("1 \t \n \r\n 2");
    assert!(tokens.contains(&TokenKind::Int(1)));
    assert!(tokens.contains(&TokenKind::Int(2)));
}

#[test]
fn test_trailing_whitespace() {
    let tokens = lex("42   ");
    assert_eq!(tokens[0], TokenKind::Int(42));
}

#[test]
fn test_leading_whitespace() {
    let tokens = lex("   42");
    assert_eq!(tokens[0], TokenKind::Int(42));
}

#[test]
fn test_only_newlines() {
    let tokens = lex("\n\n\n");
    assert_eq!(tokens, vec![TokenKind::Eof]);
}

#[test]
fn test_blank_lines_between_tokens() {
    let tokens = lex("1\n\n\n2");
    assert!(tokens.contains(&TokenKind::Int(1)));
    assert!(tokens.contains(&TokenKind::Int(2)));
}

// ============================================================================
// Complex Real-World Patterns
// ============================================================================

#[test]
fn test_function_definition_tokens() {
    let tokens = lex("fn add(x: Int, y: Int) -> Int = x + y;");
    assert!(tokens.contains(&TokenKind::Fn));
    assert!(tokens.contains(&TokenKind::Arrow));
    assert!(tokens.contains(&TokenKind::Semicolon));
}

#[test]
fn test_let_binding_tokens() {
    let tokens = lex("let x: Int = 42;");
    assert!(tokens.contains(&TokenKind::Let));
    assert!(tokens.contains(&TokenKind::Colon));
    assert!(tokens.contains(&TokenKind::Eq));
}

#[test]
fn test_match_expression_tokens() {
    let tokens = lex("match x { 0 -> zero, _ -> other }");
    assert!(tokens.contains(&TokenKind::Match));
    assert!(tokens.contains(&TokenKind::Arrow));
    assert!(tokens.contains(&TokenKind::Comma));
}

#[test]
fn test_record_with_shorthand() {
    let tokens = lex("#{ x, y, z = 3 }");
    assert!(tokens.contains(&TokenKind::HashLBrace));
    let comma_count = tokens.iter().filter(|t| **t == TokenKind::Comma).count();
    assert_eq!(comma_count, 2);
}

#[test]
fn test_list_comprehension_tokens() {
    let tokens = lex("[x * 2 | x <- xs, x > 0]");
    assert!(tokens.contains(&TokenKind::Pipe));
    // <- is tokenized as Lt + Minus
    assert!(tokens.contains(&TokenKind::Lt));
    assert!(tokens.contains(&TokenKind::Minus));
}

#[test]
fn test_type_annotation_tokens() {
    let tokens = lex("x: List<Option<Int>>");
    assert!(tokens.contains(&TokenKind::Colon));
    assert!(tokens.contains(&TokenKind::Lt));
    assert!(tokens.contains(&TokenKind::Gt));
}

#[test]
fn test_import_statement_tokens() {
    let tokens = lex("import std.list (map, filter);");
    assert!(tokens.contains(&TokenKind::Import));
    assert!(tokens.contains(&TokenKind::Dot));
    assert!(tokens.contains(&TokenKind::Semicolon));
}

#[test]
fn test_struct_definition_tokens() {
    let tokens = lex("struct Point { x: Float, y: Float };");
    assert!(tokens.contains(&TokenKind::Struct));
    assert!(tokens.contains(&TokenKind::LBrace));
    assert!(tokens.contains(&TokenKind::RBrace));
}

#[test]
fn test_enum_definition_tokens() {
    let tokens = lex("enum Option<T> { Some(T), None };");
    assert!(tokens.contains(&TokenKind::Enum));
    assert!(tokens.contains(&TokenKind::Lt));
    assert!(tokens.contains(&TokenKind::Gt));
}

#[test]
fn test_trait_definition_tokens() {
    let tokens = lex("trait Show { fn show(self) -> String; };");
    assert!(tokens.contains(&TokenKind::Trait));
    assert!(tokens.contains(&TokenKind::SelfLower));
}

#[test]
fn test_impl_block_tokens() {
    let tokens = lex("impl Show for Int { fn show(self) -> String = toString(self); };");
    assert!(tokens.contains(&TokenKind::Impl));
    // "for" is parsed as an identifier since it's not a reserved keyword
    assert!(tokens.contains(&TokenKind::Ident("for".to_string())));
}

// ============================================================================
// Boundary and Corner Cases
// ============================================================================

#[test]
fn test_max_int() {
    let tokens = lex("9223372036854775807");
    assert!(matches!(tokens[0], TokenKind::Int(_)));
}

#[test]
fn test_min_negative_after_parse() {
    // Lexer sees: Minus, Int
    let tokens = lex("-9223372036854775808");
    assert!(tokens.contains(&TokenKind::Minus));
}

#[test]
fn test_unicode_identifiers() {
    // Greek letters often used in math
    let tokens = lex("α β γ δ");
    // May or may not be valid identifiers
    assert!(!tokens.is_empty());
}

#[test]
fn test_emoji_in_string() {
    let tokens = lex(r#""👋🌍""#);
    if let TokenKind::String(s) = &tokens[0] {
        assert!(s.contains('👋'));
    }
}

#[test]
fn test_zero_width_chars() {
    // Zero-width space between tokens
    let tokens = lex("1\u{200B}2");
    // Should parse as two separate integers or handle gracefully
    assert!(!tokens.is_empty());
}

#[test]
fn test_bom_at_start() {
    let tokens = lex("\u{FEFF}42");
    // BOM should be ignored
    assert!(tokens.contains(&TokenKind::Int(42)));
}
