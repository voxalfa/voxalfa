(parameter_assignment
  name: (identifier) @property)

(language_directive
  "@" @keyword
  type: (identifier) @keyword)

(string) @string
(token) @identifier
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
  (empty_note) 
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
 "<"
 ">"
 (underline_marker)
] @punctuation.bracket

[
 "---"
 ","
] @punctuation.delimiter

[
  (medium_division)
  (normal_division)
  (half_division)
  (quarter_division)
  (concat_operator)
  (newline_operator)
  (lyric_span)
] @operator

[
 (inline_comment)
] @comment @spell

