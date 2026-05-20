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

(note
  variation: (note_variation)? @type.qualifier
  octave: (note_octave)? @number)

[
 "["
 "]"
 "{"
 "}"
 (underline_start)
 (underline_end)
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
  (lyric_break)
  (lyric_split)
  (concat_operator)
] @operator

[
 (inline_comment)
 (delimited_comment)
 (multiline_comment)
] @comment @spell

