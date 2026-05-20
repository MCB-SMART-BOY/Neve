module.exports = grammar({
  name: 'neve',

  word: $ => $.ident,

  extras: $ => [
    /\s/,
    $.comment,
  ],

  rules: {
    source_file: $ => repeat(choice(
      $.let_stmt,
      $.fn_def,
      $.import_stmt,
      $.type_alias,
      $.struct_def,
      $.enum_def,
      $.trait_def,
      $.impl_def,
      $.expr_stmt,
    )),

    comment: $ => token(seq('--', /.*/)),

    // ---- Statements ----

    let_stmt: $ => seq(
      optional('pub'),
      'let',
      $.pattern,
      optional(seq(':', $._type)),
      '=',
      $.expr,
      ';'
    ),

    fn_def: $ => seq(
      optional('pub'),
      'fn',
      $.ident,
      optional($.generics),
      '(',
      optional(seq(
        $.param,
        repeat(seq(',', $.param)),
        optional(',')
      )),
      ')',
      optional(seq('->', $._type)),
      optional('effect'),
      '=',
      $.expr,
      ';'
    ),

    param: $ => seq(
      optional('lazy'),
      $.pattern,
      ':',
      $._type,
    ),

    generics: $ => seq(
      '<',
      $.ident,
      repeat(seq(',', $.ident)),
      '>'
    ),

    import_stmt: $ => seq(
      optional('pub'),
      'import',
      $.path,
      optional(seq('as', $.ident)),
      ';'
    ),

    type_alias: $ => seq(
      optional('pub'),
      'type',
      $.ident,
      optional($.generics),
      '=',
      $._type,
      ';'
    ),

    struct_def: $ => seq(
      optional('pub'),
      'struct',
      $.ident,
      optional($.generics),
      '{',
      optional(seq(
        $.field_def,
        repeat(seq(',', $.field_def)),
        optional(',')
      )),
      '}'
    ),

    field_def: $ => seq(
      $.ident,
      ':',
      $._type,
      optional(seq('=', $.expr)),
    ),

    enum_def: $ => seq(
      optional('pub'),
      'enum',
      $.ident,
      optional($.generics),
      '{',
      optional(seq(
        $.variant,
        repeat(seq(',', $.variant)),
        optional(',')
      )),
      '}'
    ),

    variant: $ => seq(
      $.ident,
      optional(choice(
        seq('(', optional(seq($._type, repeat(seq(',', $._type)))), ')'),
        seq('#{', optional(seq($.field_def, repeat(seq(',', $.field_def)))), '}'),
      ))
    ),

    trait_def: $ => seq(
      optional('pub'),
      'trait',
      $.ident,
      optional($.generics),
      '{',
      repeat($.trait_item),
      '}'
    ),

    trait_item: $ => seq(
      'fn',
      $.ident,
      optional($.generics),
      '(',
      optional(seq($.param, repeat(seq(',', $.param)))),
      ')',
      optional(seq('->', $._type)),
      optional('effect'),
      optional(seq('=', $.expr)),
      ';'
    ),

    impl_def: $ => seq(
      optional('pub'),
      'impl',
      optional($.generics),
      optional(seq($._type, 'for')),
      $._type,
      '{',
      repeat($.impl_item),
      '}'
    ),

    impl_item: $ => seq(
      'fn',
      $.ident,
      optional($.generics),
      '(',
      optional(seq($.param, repeat(seq(',', $.param)))),
      ')',
      optional(seq('->', $._type)),
      optional('effect'),
      '=',
      $.expr,
      ';'
    ),

    expr_stmt: $ => seq($.expr, ';'),

    // ---- Patterns ----

    pattern: $ => choice(
      prec(1, $.ident),
      $._wildcard,
      $.literal_pattern,
      $.tuple_pattern,
      $.list_pattern,
      $.record_pattern,
      $.constructor_pattern,
      $.or_pattern,
      $.binding_pattern,
    ),

    _wildcard: $ => '_',

    literal_pattern: $ => choice(
      $.number,
      $.string,
      'true',
      'false',
    ),

    tuple_pattern: $ => seq(
      '(',
      optional(seq($.pattern, repeat(seq(',', $.pattern)))),
      ')'
    ),

    list_pattern: $ => seq(
      '[',
      optional(seq($.pattern, repeat(seq(',', $.pattern)))),
      ']'
    ),

    record_pattern: $ => seq(
      '#{',
      optional(seq(
        $.record_pattern_field,
        repeat(seq(',', $.record_pattern_field)),
      )),
      '}'
    ),

    record_pattern_field: $ => seq(
      $.ident,
      optional(seq('=', $.pattern)),
    ),

    constructor_pattern: $ => prec.dynamic(-1, seq(
      $.path,
      optional(seq('(', optional(seq($.pattern, repeat(seq(',', $.pattern)))), ')'))
    )),

    or_pattern: $ => prec.left(seq($.pattern, '|', $.pattern)),

    binding_pattern: $ => prec(2, seq($.ident, '@', $.pattern)),

    // ---- Expressions ----

    expr: $ => choice(
      $.literal,
      $.path,
      $.call,
      $.method_call,
      $.field_access,
      $.index_expr,
      $.if_expr,
      $.match_expr,
      $.block,
      $.list,
      $.record,
      $.tuple,
      $.lambda,
      $.unary,
      $.pipe,
      $.binary,
      $.lazy_expr,
    ),

    literal: $ => choice(
      $.number,
      $.string,
      $.char,
      $.interpolated,
      $.path_literal,
      'true',
      'false',
      '()',
    ),

    number: $ => token(/[0-9][0-9_]*(\.[0-9]+)?/),
    string: $ => token(seq('"', /[^"]*/, '"')),
    char: $ => token(seq("'", /[^']/, "'")),

    interpolated: $ => seq(
      '`',
      repeat(choice(
        /[^`{]+/,
        seq('{', $.expr, '}'),
      )),
      '`'
    ),

    path_literal: $ => token(/(\.\/|\.\.\/|\/)[a-zA-Z0-9_\-\.\/]*/),

    path: $ => prec.left(seq($.ident, repeat(seq('.', $.ident)))),

    ident: $ => token(/[a-zA-Z_][a-zA-Z0-9_]*/),

    // ---- Operators with proper precedence ----

    call: $ => prec(1, seq(
      $.path,
      '(',
      optional(seq($.expr, repeat(seq(',', $.expr)))),
      ')'
    )),

    method_call: $ => prec(1, seq(
      $.expr,
      '.',
      $.ident,
      '(',
      optional(seq($.expr, repeat(seq(',', $.expr)))),
      ')'
    )),

    field_access: $ => prec(1, seq(
      $.expr,
      '.',
      $.ident,
    )),

    index_expr: $ => prec(1, seq(
      $.expr,
      '[',
      $.expr,
      ']'
    )),

    unary: $ => prec(2, seq(
      choice('-', '!'),
      $.expr,
    )),

    pipe: $ => prec.right(3, seq(
      $.expr,
      '|>',
      $.expr,
    )),

    binary: $ => prec.left(4, seq(
      $.expr,
      choice(
        '+', '-', '*', '/', '%', '^',
        '==', '!=', '<', '>', '<=', '>=',
        '&&', '||',
        '++', '//',
        '??', '?.',
      ),
      $.expr,
    )),

    if_expr: $ => prec(5, seq(
      'if',
      $.expr,
      'then',
      $.expr,
      'else',
      $.expr,
    )),

    match_expr: $ => prec(5, seq(
      'match',
      $.expr,
      '{',
      optional(seq(
        $.match_arm,
        repeat(seq(',', $.match_arm)),
        optional(',')
      )),
      '}'
    )),

    match_arm: $ => seq(
      $.pattern,
      optional(seq('if', $.expr)),
      '->',
      $.expr,
    ),

    block: $ => seq(
      '{',
      repeat($.stmt),
      optional($.expr),
      '}'
    ),

    stmt: $ => choice(
      $.let_stmt,
      $.expr_stmt,
      $.import_stmt,
    ),

    list: $ => seq(
      '[',
      optional(seq($.expr, repeat(seq(',', $.expr)))),
      ']'
    ),

    record: $ => seq(
      '#{',
      optional(seq(
        $.ident,
        '=',
        $.expr,
        repeat(seq(',', $.ident, '=', $.expr)),
      )),
      '}'
    ),

    tuple: $ => seq(
      '(',
      optional(seq($.expr, repeat(seq(',', $.expr)))),
      ')'
    ),

    lambda: $ => seq(
      'fn',
      '(',
      optional(seq($.param, repeat(seq(',', $.param)))),
      ')',
      optional(seq('->', $._type)),
      '=>',
      $.expr,
    ),

    lazy_expr: $ => seq('lazy', $.expr),

    // ---- Types ----

    _type: $ => choice(
      $.named_type,
      $.fn_type,
      $.tuple_type,
      $.record_type,
    ),

    named_type: $ => prec.left(seq(
      $.ident,
      repeat(seq('.', $.ident)),
      optional(seq('<', optional(seq($._type, repeat(seq(',', $._type)))), '>')),
    )),

    fn_type: $ => seq(
      'fn',
      '(',
      optional(seq($._type, repeat(seq(',', $._type)))),
      ')',
      optional(seq('->', $._type)),
    ),

    tuple_type: $ => seq(
      '(',
      optional(seq($._type, repeat(seq(',', $._type)))),
      ')'
    ),

    record_type: $ => seq(
      '#{',
      optional(seq(
        $.ident, ':', $._type,
        repeat(seq(',', $.ident, ':', $._type)),
      )),
      '}'
    ),
  }
});
