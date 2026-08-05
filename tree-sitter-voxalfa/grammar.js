/**
 * @file Voxalfa tree-sitter parser
 * @author LIOKA Ranarison Fiderana <luckasranarison@gmail.com>
 * @license Apache-2.0
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "voxalfa",

  extras: ($) => [$.inline_comment, $._space],

  rules: {
    source_file: ($) =>
      seq(optional($.header), optional(seq($.header_delimiter, $.body))),

    header: ($) => repeat1($._header_line),
    header_delimiter: () => "---",

    _header_line: ($) =>
      choice(
        $.language_directive,
        $.metadata_line,
        $.parameter_line,
        $._linebreak,
      ),

    body: ($) => sep1($._section_separator, $.section),

    _section_separator: ($) =>
      choice(
        $.section_split_delimiter,
        $.section_merge_delimiter,
        $.section_major_delimiter,
      ),

    section_split_delimiter: () => "--",
    section_merge_delimiter: () => "<<",
    section_major_delimiter: () => "==",
    sub_section_delimiter: () => "++",

    section: ($) => sep1($.sub_section_delimiter, $.sub_section),

    sub_section: ($) => repeat1($._section_line),

    _section_line: ($) =>
      choice(
        $.language_directive,
        $.metadata_line,
        $.parameter_line,
        $.solfa_line,
        $.lyric_line,
        $._linebreak,
      ),

    _kv_separator: () => "|",

    metadata_line: ($) =>
      prec.right(
        seq(seq("[", "#", "]"), sep1($._kv_separator, $.parameter_assignment)),
      ),

    parameter_line: ($) =>
      prec.right(
        seq(seq("[", "$", "]"), sep1($._kv_separator, $.parameter_assignment)),
      ),

    dynamics_line: ($) =>
      prec.right(
        seq(seq("[", "^", "]"), sep1($._kv_separator, $.parameter_assignment)),
      ),

    solfa_line: ($) =>
      seq(
        "[",
        field("voice", $.identifier),
        "]",
        field("content", $.solfa_content),
        "||",
      ),

    lyric_line: ($) =>
      seq(
        seq("[", field("verse", $.integer), "]"),
        field("content", $.lyric_content),
        field("anchor", optional($.lyric_anchor)),
      ),

    parameter_assignment: ($) =>
      seq(
        field("name", $.identifier),
        "=",
        choice(field("value", $.string), $._delimited_value),
      ),

    identifier: () => /[a-zA-Z][a-zA-Z_-]*/,

    _delimited_value: ($) =>
      seq(
        "{",
        field("value", choice($._value_primitive, $.list, $.timed_value)),
        "}",
      ),

    _value_primitive: ($) =>
      seq(choice($._number, $.string, $.boolean, $.builtin)),
    _value_structured: ($) => choice($._value_primitive, $.timed_value),

    list: ($) => seq($._value_structured, ",", sep1(",", $._value_structured)),

    string: ($) => seq('"', $.string_content, '"'),

    builtin: () => /[a-zA-Z#]+/,

    timed_value: ($) =>
      seq(
        field("value", $._value_primitive),
        ":",
        field("start", $._number),
        optional(seq("..", field("end", $._number))),
      ),

    string_content: () => /[^"\n]*/,

    inline_string: () => /[^\n]+/,

    boolean: () => choice("true", "false"),
    integer: () => /\d+/,
    float: ($) => seq(optional($.integer), ".", $.integer),
    _number: ($) => prec.right(choice($.float, $.integer)),

    _accent: ($) => choice($.strong_accent, $.medium_accent, $.weak_accent),

    strong_accent: () => "|",
    medium_accent: () => "!",
    weak_accent: () => ":",

    solfa_content: ($) => repeat1($.pulse),

    pulse: ($) =>
      seq(
        field("accent", $._accent),
        field("tokens", optional($.pulse_tokens)),
      ),

    pulse_tokens: ($) =>
      repeat1(
        choice(
          $.half_division,
          $.quarter_division,
          $.underline_marker,
          $.prolonged_note,
          $.note,
        ),
      ),

    half_division: () => ".",
    quarter_division: () => ",",
    underline_marker: () => "`",
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
            $._lyric_operator,
            choice($.lyric_column, blank()), // FIXME: blank() is used to allow trailing spaces and concat
          ),
        ),
      ),

    lyric_anchor: () => "@@",

    _lyric_operator: ($) =>
      choice($.concat_operator, $.space_operator, $.newline_operator),

    lyric_column: ($) =>
      seq(
        field(
          "lyric",
          choice($.lyric_group, $.lyric_chunk, $.lyric_placeholder),
        ),
        field("span", optional($.lyric_span)),
      ),

    lyric_group: ($) =>
      seq(
        "(",
        sep1(choice($.space_operator, $.newline_operator), $.lyric_chunk),
        ")",
      ),

    lyric_chunk: ($) =>
      repeat1(choice($.lyric_string, $.lyric_special, $.underline_marker)),

    lyric_span: ($) => seq("@", field("count", $.integer)),

    space_operator: () => / +/,
    concat_operator: () => /_+/,
    newline_operator: () => /\\+/,

    lyric_string: () => /[^\s_/~``<>\\/\()@&;]+(\.[^\s_/~``<>\\/\()@&;\.]+)?/,
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
        "&atr", // @
        "&amp", // &
        "&scl", // ;
        "&dot", // .
      ),

    _space: () => /[ \t]+/,
    _linebreak: () => /[\n]+/,

    language_directive: ($) =>
      seq(
        ";;",
        "@",
        field("name", $.identifier),
        $._space,
        field("value", $.inline_string),
      ),

    inline_comment: (_) => seq(";", /[^\n]*/),
  },
});

/**
 * @param {RuleOrLiteral} separator
 * @param {RuleOrLiteral} node
 */
function sep1(separator, node) {
  return seq(node, repeat(seq(separator, node)));
}
