#[derive(Debug, Clone, Copy)]
pub struct BuiltinValueSpec {
    pub label: &'static str,
    pub detail: &'static str,
    pub doc: &'static str,
}

pub const VOICE_BUILTINS: &[BuiltinValueSpec] = &[
    BuiltinValueSpec {
        label: "S",
        detail: "Soprano",
        doc: "Highest female voice part.",
    },
    BuiltinValueSpec {
        label: "A",
        detail: "Alto",
        doc: "Lowest female voice part.",
    },
    BuiltinValueSpec {
        label: "T",
        detail: "Tenor",
        doc: "Highest male voice part.",
    },
    BuiltinValueSpec {
        label: "B",
        detail: "Bass",
        doc: "Lowest male voice part.",
    },
];

pub const KEY_BUILTINS: &[BuiltinValueSpec] = &[
    BuiltinValueSpec {
        label: "C",
        detail: "C Major / A Minor",
        doc: "Key signature with 0 sharps/flats.",
    },
    BuiltinValueSpec {
        label: "G",
        detail: "G Major / E Minor",
        doc: "Key signature with 1 sharp (F#).",
    },
    BuiltinValueSpec {
        label: "D",
        detail: "D Major / B Minor",
        doc: "Key signature with 2 sharps (F#, C#).",
    },
    BuiltinValueSpec {
        label: "A",
        detail: "A Major / F# Minor",
        doc: "Key signature with 3 sharps (F#, C#, G#).",
    },
    BuiltinValueSpec {
        label: "E",
        detail: "E Major / C# Minor",
        doc: "Key signature with 4 sharps.",
    },
    BuiltinValueSpec {
        label: "B",
        detail: "B Major / G# Minor",
        doc: "Key signature with 5 sharps.",
    },
    BuiltinValueSpec {
        label: "F#",
        detail: "F# Major / D# Minor",
        doc: "Key signature with 6 sharps.",
    },
    BuiltinValueSpec {
        label: "F",
        detail: "F Major / D Minor",
        doc: "Key signature with 1 flat (Bb).",
    },
    BuiltinValueSpec {
        label: "Bb",
        detail: "Bb Major / G Minor",
        doc: "Key signature with 2 flats (Bb, Eb).",
    },
    BuiltinValueSpec {
        label: "Eb",
        detail: "Eb Major / C Minor",
        doc: "Key signature with 3 flats.",
    },
    BuiltinValueSpec {
        label: "Ab",
        detail: "Ab Major / F Minor",
        doc: "Key signature with 4 flats.",
    },
    BuiltinValueSpec {
        label: "Db",
        detail: "Db Major / Bb Minor",
        doc: "Key signature with 5 flats.",
    },
    BuiltinValueSpec {
        label: "Gb",
        detail: "Gb Major / Eb Minor",
        doc: "Key signature with 6 flats.",
    },
];

pub const TEMPO_BUILTINS: &[BuiltinValueSpec] = &[
    BuiltinValueSpec {
        label: "grave",
        detail: "grave",
        doc: "Extremely slow tempo (~40 BPM).",
    },
    BuiltinValueSpec {
        label: "largo",
        detail: "largo",
        doc: "Broad, slow tempo (~50 BPM).",
    },
    BuiltinValueSpec {
        label: "adagio",
        detail: "adagio",
        doc: "Slow tempo with expression (~70 BPM).",
    },
    BuiltinValueSpec {
        label: "andante",
        detail: "adante",
        doc: "Walking tempo (~90 BPM).",
    },
    BuiltinValueSpec {
        label: "moderato",
        detail: "moderato",
        doc: "Moderate speed (~110 BPM).",
    },
    BuiltinValueSpec {
        label: "allegro",
        detail: "allegro",
        doc: "Fast, lively tempo (~130 BPM).",
    },
    BuiltinValueSpec {
        label: "vivace",
        detail: "vivace",
        doc: "Brisk, lively tempo (~160 BPM).",
    },
    BuiltinValueSpec {
        label: "presto",
        detail: "presto",
        doc: "Very fast tempo (~180 BPM).",
    },
];

pub const MARK_BUILTINS: &[BuiltinValueSpec] = &[
    BuiltinValueSpec {
        label: "S",
        detail: "Segno",
        doc: "Section sign marker used for navigation repeats.",
    },
    BuiltinValueSpec {
        label: "C",
        detail: "Coda",
        doc: "Coda section marker indicating the concluding passage.",
    },
    BuiltinValueSpec {
        label: "TC",
        detail: "To Coda",
        doc: "Directive marking the jump-off point to the Coda.",
    },
    BuiltinValueSpec {
        label: "F",
        detail: "Fine",
        doc: "Marks the formal end of the piece.",
    },
];

pub const TOUCHES_BUILTINS: &[BuiltinValueSpec] = &[
    BuiltinValueSpec {
        label: "stc",
        detail: "Staccato",
        doc: "Short, detached note articulation.",
    },
    BuiltinValueSpec {
        label: "acc",
        detail: "Accent",
        doc: "Emphasis on note attack.",
    },
    BuiltinValueSpec {
        label: "frm",
        detail: "Fermata",
        doc: "Sustain / hold note beyond standard duration.",
    },
];

pub const DYNAMICS_BUILTINS: &[BuiltinValueSpec] = &[
    BuiltinValueSpec {
        label: "ppp",
        detail: "Pianississimo",
        doc: "Extremely soft dynamic mark.",
    },
    BuiltinValueSpec {
        label: "pp",
        detail: "Pianissimo",
        doc: "Very soft dynamic mark.",
    },
    BuiltinValueSpec {
        label: "p",
        detail: "Piano",
        doc: "Soft dynamic mark.",
    },
    BuiltinValueSpec {
        label: "mp",
        detail: "Mezzo-piano",
        doc: "Moderately soft dynamic mark.",
    },
    BuiltinValueSpec {
        label: "mf",
        detail: "Mezzo-forte",
        doc: "Moderately loud dynamic mark.",
    },
    BuiltinValueSpec {
        label: "f",
        detail: "Forte",
        doc: "Loud dynamic mark.",
    },
    BuiltinValueSpec {
        label: "ff",
        detail: "Fortissimo",
        doc: "Very loud dynamic mark.",
    },
    BuiltinValueSpec {
        label: "fff",
        detail: "Fortississimo",
        doc: "Extremely loud dynamic mark.",
    },
];

pub const JUMP_BUILTINS: &[BuiltinValueSpec] = &[
    BuiltinValueSpec {
        label: "DS",
        detail: "Dal Segno",
        doc: "Repeat playback starting from the Segno mark [S].",
    },
    BuiltinValueSpec {
        label: "DC",
        detail: "Da Capo",
        doc: "Repeat playback from the very beginning.",
    },
    BuiltinValueSpec {
        label: "DSC",
        detail: "Dal Segno al Coda",
        doc: "Repeat from Segno [S], then jump to Coda [C] at To Coda [TC].",
    },
    BuiltinValueSpec {
        label: "DSF",
        detail: "Dal Segno al Fine",
        doc: "Repeat from Segno [S] and end at Fine [F].",
    },
    BuiltinValueSpec {
        label: "DCC",
        detail: "Da Capo al Coda",
        doc: "Repeat from start, then jump to Coda [C] at To Coda [TC].",
    },
    BuiltinValueSpec {
        label: "DCF",
        detail: "Da Capo al Fine",
        doc: "Repeat from start and end at Fine [F].",
    },
];
