/**
 * @file Voxalfa tree-sitter parser
 * @author LIOKA Ranarison Fiderana <luckasranarison@gmail.com>
 * @license Apache-2.0
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "voxalfa",

  extras: ($) => [$.inline_comment],

  rules: {
    source_file: ($) => seq(optional($.header), "---", optional($.body)),

    header: ($) => repeat1($._header_line),

    _header_line: ($) =>
      choice($.metadata_line, $.parameter_line, $._multispace),

    body: ($) => sep1("--", $.section),

    section: ($) => repeat1($._section_line),

    _section_line: ($) =>
      choice(
        $._multispace,
        $.parameter_line,
        $.dynamics_line,
        $.solfa_line,
        $.lyric_line,
      ),

    metadata_line: ($) =>
      seq(
        seq("[", "#", "]"),
        sep(
          "|",
          seq(optional($._space), $.parameter_assignment, optional($._space)),
        ),
      ),

    parameter_line: ($) =>
      seq(
        seq("[", "$", "]"),
        sep(
          "|",
          seq(optional($._space), $.parameter_assignment, optional($._space)),
        ),
      ),

    dynamics_line: ($) =>
      seq(
        seq("[", "^", "]"),
        sep(
          "|",
          seq(optional($._space), $.parameter_assignment, optional($._space)),
        ),
      ),

    solfa_line: ($) =>
      seq(
        "[",
        field("voice", $.token),
        "]",
        optional($._space),
        repeat1($.pulse),
        "||",
      ),

    lyric_line: ($) =>
      seq(
        seq("[", field("verse", $.integer), "]"),
        optional($._space),
        field("prefix", $._lyric_prefix),
        optional($._space),
        field("content", $.lyric_content),
      ),

    parameter_assignment: ($) =>
      seq(
        field("name", $.identifier),
        optional($._space),
        "=",
        optional($._space),
        choice(field("value", $.string), $._delimited_value),
      ),

    identifier: () => /[a-zA-Z_$-]+/,

    _delimited_value: ($) =>
      seq("{", field("value", choice($._value_atom, $.list)), "}"),
    _value_atom: ($) =>
      seq(
        optional($._space),
        choice($._number, $.token, $.string, $.boolean),
        optional($._space),
      ),

    list: ($) => seq($._value_atom, ",", sep1(",", $._value_atom)),

    string: ($) => seq('"', $.string_content, '"'),
    string_content: () => /[^"\n]*/,
    inline_string: () => /[^\n]+/,

    token: () => /[a-zA-Z#]+/,
    boolean: () => choice("true", "false"),
    integer: () => /\d+/,
    float: ($) => seq(optional($.integer), ".", $.integer),
    _number: ($) => prec.right(choice($.float, $.integer)),

    _accent: ($) => choice($.strong_accent, $.medium_accent, $.weak_accent),

    strong_accent: () => "|",
    medium_accent: () => "!",
    weak_accent: () => ":",

    pulse: ($) =>
      seq(field("accent", $._accent), field("tokens", $.pulse_tokens)),

    pulse_tokens: ($) =>
      repeat1(
        choice(
          $._space,
          $.half_division,
          $.quarter_division,
          $.underline_marker,
          $.empty_note,
          $.prolonged_note,
          $.note,
        ),
      ),

    half_division: () => ".",
    quarter_division: () => ",",
    underline_marker: () => "`",
    empty_note: () => "~",
    prolonged_note: () => "-",

    note: ($) =>
      seq(
        field("base", $.note_base),
        field("variation", optional($.note_variation)),
        field("octave", optional($.note_octave)),
      ),

    note_octave: () => /[+-][\d]/,
    note_base: () => /[drmfslt]/,
    note_variation: () => /[ai]/,

    lyric_content: ($) =>
      seq(
        $.lyric_column,
        repeat(
          seq(
            choice($.concat_operator, $.space_operator, $.newline_operator),
            choice($.lyric_column), // FIXME: blank() is used to allow trailing spaces and concat
          ),
        ),
      ),

    _lyric_prefix: ($) =>
      choice($.space_prefix, $.concat_prefix, $.newline_prefix),

    space_prefix: () => "=",
    concat_prefix: () => "<",
    newline_prefix: () => ">",

    lyric_column: ($) =>
      seq(
        field("lyric", choice($.lyric_group, $.lyric_chunk)),
        field("span", optional($.lyric_span)),
      ),

    lyric_chunk: ($) =>
      seq(
        optional($.underline_marker),
        choice($._lyric_string, $.lyric_placeholder),
        optional($.underline_marker),
      ),

    lyric_group: ($) =>
      seq(
        "(",
        sep1(
          choice($.space_operator, $.newline_operator),
          seq(
            optional($.underline_marker),
            $._lyric_string,
            optional($.underline_marker),
          ),
        ),
        ")",
      ),

    _lyric_string: ($) => repeat1(choice($.lyric_string, $.lyric_special)),

    lyric_span: () => /\++/,

    space_operator: () => / +/,
    concat_operator: () => /_+/,
    newline_operator: () => /\\+/,

    lyric_string: () => /[^\s_/~``<>\\/\()+&;]+/,
    lyric_placeholder: () => "~",

    lyric_special: () =>
      choice(
        "&bls", // \
        "&tld", // ~
        "&btk", // `
        "&lch", // <
        "&rch", // >
        "&sls", // /
        "&lpr", // (
        "&rpr", // )
        "&pls", // +
        "&amp", // &
        "&scl", // ;
      ),

    _space: () => /[ \t]+/,
    _multispace: () => /[ \t]*[\n]+[ \t]*/,

    language_directive: ($) =>
      seq(
        "@",
        field("type", $.identifier),
        $._space,
        field("value", $.inline_string),
      ),

    inline_comment: ($) =>
      seq(";", /[ \t]*/, choice(/[^@][^\n]*/, $.language_directive)),
  },
});

/**
 * @param {RuleOrLiteral} separator
 * @param {RuleOrLiteral} node
 */
function sep(separator, node) {
  return optional(sep1(separator, node));
}

/**
 * @param {RuleOrLiteral} separator
 * @param {RuleOrLiteral} node
 */
function sep1(separator, node) {
  return seq(node, repeat(seq(separator, node)));
}
