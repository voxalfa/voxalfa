/**
 * @file Voxalfa tree-sitter parser
 * @author LIOKA Ranarison Fiderana <luckasranarison@gmail.com>
 * @license Apache-2.0
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "voxalfa",

  extras: ($) => [
    $._multispace,
    $.inline_comment,
    $.delimited_comment,
    $.multiline_comment,
  ],

  rules: {
    source_file: ($) => seq(optional($.header), "---", optional($.body)),

    header: ($) => repeat1($._header_line),

    _header_line: ($) => choice($.metadata_line, $.parameter_line),

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
      seq(seq("[", field("voice", $.token), "]"), sep1("|", $.measure)),

    lyric_line: ($) =>
      seq(
        seq("[", field("verse", $.integer), "]"),
        optional($._space),
        sep1(
          choice($.concat_operator, $.space_operator),
          choice($.lyric_chunk, $.lyric_placeholder, blank()),
        ),
      ),

    parameter_assignment: ($) =>
      seq(
        field("name", $.identifier),
        optional($._space),
        "=",
        optional($._space),
        field("value", choice($.string, $._delimited_value)),
      ),

    identifier: () => /[a-zA-Z_-]+/,

    _delimited_value: ($) =>
      prec(1, seq("{", choice($._value_atom, $.list), "}")),

    _value_atom: ($) =>
      seq(
        optional($._space),
        choice($._number, $.token, $.string, $.boolean),
        optional($._space),
      ),

    list: ($) => sep1(",", $._value_atom),

    string: () => seq('"', field("value", /[^"\n]*/), '"'),
    inline_string: () => /[^\n]+/,

    token: () => /[a-zA-Z#]+/,
    boolean: () => choice("true", "false"),
    integer: () => /\d+/,
    float: ($) => seq(optional($.integer), ".", $.integer),
    _number: ($) => prec.right(choice($.float, $.integer)),

    measure: ($) =>
      sep1(
        choice($.medium_division, $.normal_division),
        sep1(
          repeat1(choice($.half_division, $.quarter_division)),
          seq(
            optional($._space),
            optional($.underline_start),
            optional($._space),
            $.pulse,
            optional($._space),
            optional($.underline_end),
            optional($._space),
          ),
        ),
      ),

    medium_division: () => "!",
    normal_division: () => ":",
    half_division: () => ".",
    quarter_division: () => ",",

    underline_start: () => "<",
    underline_end: () => ">",

    pulse: ($) => choice($.empty_note, $.prolonged_note, $.note),

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

    lyric_chunk: ($) =>
      repeat1(
        choice(
          $.lyric_string,
          $.lyric_break,
          $.lyric_split,
          $.underline_start,
          $.underline_end,
        ),
      ),

    space_operator: () => / +/,
    concat_operator: () => /_+/,

    lyric_string: () => /[^\s_/~``<>\\/]+/,
    lyric_break: () => "\\",
    lyric_split: () => "/",
    lyric_placeholder: () => "~",

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
      seq("~~", choice(/[ \t]*[^@][^\n]*/, $.language_directive)),

    delimited_comment: () => seq("(", /[^)\n]*/, ")"),
    multiline_comment: () => /~~~[^~]*~~~/,
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
