#[derive(Debug, Clone, Copy)]
pub struct ParamSpec {
    pub name: &'static str,
    pub type_str: &'static str,
    pub snippet: &'static str,
    pub doc: &'static str,
}

pub const HEADER_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "title",
        type_str: "string",
        snippet: "title=\"${1:value}\"",
        doc: "Header metadata field specifying the song title.",
    },
    ParamSpec {
        name: "author",
        type_str: "{string...}",
        snippet: "author={\"${1:value}\"}",
        doc: "Header metadata field specifying the author(s) or lyricist(s).",
    },
    ParamSpec {
        name: "composer",
        type_str: "{string...}",
        snippet: "composer={\"${1:value}\"}",
        doc: "Header metadata field listing composer(s) of the piece.",
    },
    ParamSpec {
        name: "verses",
        type_str: "integer",
        snippet: "verses={${1:1}}",
        doc: "Header metadata field indicating the total number of verses.",
    },
    ParamSpec {
        name: "meter",
        type_str: "{number...}",
        snippet: "meter={${1:4}}",
        doc: "Header metadata field defining poetic or rhythmic meter structure.",
    },
    ParamSpec {
        name: "description",
        type_str: "string",
        snippet: "description=\"${1:value}\"",
        doc: "Header metadata field providing additional background or notes.",
    },
    ParamSpec {
        name: "release",
        type_str: "integer",
        snippet: "release={${1:2026}}",
        doc: "Header metadata field specifying the publication or release year.",
    },
    ParamSpec {
        name: "language",
        type_str: "string",
        snippet: "language=\"${1:en}\"",
        doc: "Header metadata field setting the primary language.",
    },
    ParamSpec {
        name: "tags",
        type_str: "{string...}",
        snippet: "tags={\"${1:tag}\"}",
        doc: "Header metadata field containing categorization tags.",
    },
];

pub const INITIAL_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "key",
        type_str: "key",
        snippet: "key={${1:C}}",
        doc: "Specifies the initial musical key signature.",
    },
    ParamSpec {
        name: "time",
        type_str: "{integer,integer}",
        snippet: "time={${1:4},${2:4}}",
        doc: "Sets initial time signature as numerator and denominator.",
    },
    ParamSpec {
        name: "tempo",
        type_str: "tempo | integer",
        snippet: "tempo={${1:allegro}}",
        doc: "Sets initial playback tempo as a named mark or BPM.",
    },
    ParamSpec {
        name: "voices",
        type_str: "{voice...}",
        snippet: "voices={${1:S}}",
        doc: "Defines active voice parts for the score.",
    },
];

pub const SECTION_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "time",
        type_str: "{integer,integer}",
        snippet: "time={${1:4},${2:4}}",
        doc: "Sets section-specific time signature.",
    },
    ParamSpec {
        name: "tempo",
        type_str: "tempo | integer",
        snippet: "tempo={${1:Allegro}}",
        doc: "Changes section-level playback tempo.",
    },
    ParamSpec {
        name: "label",
        type_str: "string",
        snippet: "label=\"${1:Section}\"",
        doc: "Assigns a custom label or name to a section.",
    },
    ParamSpec {
        name: "ending",
        type_str: "integer",
        snippet: "ending={${1:1}}",
        doc: "Indicates repeated section ending index (e.g., 1st or 2nd ending).",
    },
    ParamSpec {
        name: "key",
        type_str: "key",
        snippet: "key={${1:C}}",
        doc: "Changes key signature for this section.",
    },
    ParamSpec {
        name: "jump",
        type_str: "jump",
        snippet: "jump={${1:DS}}",
        doc: "Navigation directive for repeating sections (e.g., DS, DC, Fine).",
    },
    ParamSpec {
        name: "mark",
        type_str: "mark",
        snippet: "mark={${1:S}}",
        doc: "Places a visual or functional musical marker (e.g., Segno, Coda).",
    },
    ParamSpec {
        name: "dynamics",
        type_str: "dynamic",
        snippet: "dynamics={${1:f}}",
        doc: "Sets section performance dynamics (e.g., f, p, mf).",
    },
    ParamSpec {
        name: "touches",
        type_str: "{touch...}",
        snippet: "touches={${1:stc}}",
        doc: "Applies articulation patterns across notes.",
    },
    ParamSpec {
        name: "repeat",
        type_str: "integer",
        snippet: "repeat={${1:2}}",
        doc: "Defines how many times the section repeats.",
    },
];
