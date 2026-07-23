(parameter_assignment
  name: (identifier) @property)

(language_directive
  "@" @keyword
  type: (identifier) @keyword)

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
  "^"
] @keyword

[
  (prolonged_note)
  (lyric_placeholder)
] @constant.builtin

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
 (language_directive)
] @comment @spell

