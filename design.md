# Rendering design

The voxalfa libraries are based around the **voxlfa-core** output type. The body of the partition is the most important part so it is important to understand its shape to use it for different purposes (rendering, playback, ...).

The overall structure of the body is explained in the overview, but to implement the svg rendering it is also important to understand the goal and constraints.

## Goals

- Pixel perfect SVG based rendering
- Fully customizable output (font size, font familly, ...)

## Constraints

- Output on virtual A4 paper size with pagination if it exceeds the height

- SVG elements have to be manipulated using virtual layout (taffy) so text wrap has to be done manually for rendering headers like title.

- Sections don't reflect the visual aspect of the partition, they are arbitrary blocks for composing the file. To render the notes we have to resolve the measure (relative to the time signature) and notes that fit into a single line accross multiple sections. So we must develop a traversal technique for the output section types.

- Lyrics also must be resolved with the notes, per-voice lyrics make it more difficult

- Markers like dynamics, note/key changes must be aligned with the corresponding notes but they are relative to section indexing.

- Grid based vs arbitrary offset. If we use evenly spaced grid with a 3/4 note (using ., divider) a unit column will be huge and spacing will be bigger than it should but it'd make lyrics and marker alignment easy. On the other hand if we use arbitrary offsets it'll render better, but it is more involved (the latter is preferable)
