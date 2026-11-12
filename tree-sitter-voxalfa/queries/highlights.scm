(parameter_assignment
  name: (identifier) @property)

(language_directive
  ";;" @comment
  "@" @keyword
  name: (identifier) @keyword)

(language_directive
  value: (inline_string) @comment)

(solfa_line
  voice: (identifier) @identifier)

(string) @string
(boolean) @boolean

[
  (float)
  (integer)
] @number

[
  "#"
  "$"
  (verse_wildcard)
] @keyword

[
  (prolonged_note)
  (lyric_placeholder)
] @constant.builtin

(builtin) @variable

(lyric_special) @character.special

(note
  variation: (note_variation)? @type.qualifier
  octave: (note_octave)? @number)

[
 "["
 "]"
 "{"
 "}"
 "("
 ")"
 (underline_marker)
] @punctuation.bracket

[
 ","
] @punctuation.delimiter

[
  (half_division)
  (quarter_division)
  (concat_operator)
  (newline_operator)
] @operator

(lyric_span
  "@" @operator)

[
 (inline_comment)
] @comment @spell

