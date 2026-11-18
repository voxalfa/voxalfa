use serde::Serialize;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScorePartwise {
    #[serde(rename = "@version")]
    pub version: MusicXmlVersion,
    pub part_list: PartList,
    pub part: Vec<Part>,
}

#[derive(Debug, Default, Serialize)]
pub enum MusicXmlVersion {
    #[serde(rename = "4.0")]
    #[default]
    V4,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PartList {
    pub score_part: Vec<ScorePart>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScorePart {
    #[serde(rename = "@id")]
    pub id: String,
    pub part_name: String,
}

#[derive(Debug, Serialize)]
pub struct Part {
    #[serde(rename = "@id")]
    pub id: String,
    pub measure: Vec<Measure>,
}

#[derive(Debug, Serialize)]
pub struct Measure {
    #[serde(rename = "@number")]
    pub number: usize,
    #[serde(rename = "@implicit")]
    pub implicit: Choice,
    #[serde(rename = "$value")]
    pub items: Vec<MeasureItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasureItem {
    Attributes(Attributes),
    Note(Note),
    Direction(Direction),
}

#[derive(Debug, Default, Serialize)]
pub struct Attributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divisions: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clef: Option<Clef>,
}

#[derive(Debug, Serialize)]
pub struct Key {
    pub fifths: i8, // negative flat | positive sharp
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Time {
    pub beats: u8,
    pub beat_type: u8,
}

#[derive(Debug, Serialize)]
pub enum ClefSign {
    G,
    F,
    C,
}

#[derive(Debug, Serialize)]
pub struct Clef {
    pub sign: ClefSign,
}

#[derive(Debug, Default, Serialize)]
pub struct Note {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<Pitch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest: Option<Rest>,
    pub duration: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tie: Vec<Tie>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notations: Option<Notations>,
    // pub r#type: NoteType,
}

impl Note {
    pub fn add_tie(&mut self, position: Position) {
        self.tie.push(Tie { r#type: position });

        self.notations
            .get_or_insert_with(Notations::default)
            .tied
            .push(Tie { r#type: position });
    }
}

#[derive(Debug, Serialize)]
pub struct Tie {
    #[serde(rename = "@type")]
    pub r#type: Position,
}

#[derive(Debug, Default, Serialize)]
pub struct Notations {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tied: Vec<Tie>,
}

#[derive(Debug, Serialize)]
pub struct Rest {}

#[derive(Debug, Serialize)]
pub struct Pitch {
    pub step: Step,
    pub alter: i8,
    pub octave: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Choice {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Start,
    Stop,
}

impl From<bool> for Choice {
    fn from(value: bool) -> Self {
        if value { Choice::Yes } else { Choice::No }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Step {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl From<u8> for Step {
    fn from(value: u8) -> Self {
        match value % 7 {
            0 => Self::C,
            1 => Self::D,
            2 => Self::E,
            3 => Self::F,
            4 => Self::G,
            5 => Self::A,
            _ => Self::B,
        }
    }
}

impl Step {
    pub fn semitone_offset(self) -> u8 {
        match self {
            Step::C => 0,
            Step::D => 2,
            Step::E => 4,
            Step::F => 5,
            Step::G => 7,
            Step::A => 9,
            Step::B => 11,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Direction {
    pub sound: Sound,
}

#[derive(Debug, Default, Serialize)]
pub struct Sound {
    #[serde(rename = "@segno", skip_serializing_if = "Option::is_none")]
    pub segno: Option<usize>,
    #[serde(rename = "@dalsegno", skip_serializing_if = "Option::is_none")]
    pub dalsegno: Option<usize>,
}
