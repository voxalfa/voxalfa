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
}

#[derive(Debug, Serialize)]
pub struct Attributes {
    pub divisions: u16,
    pub key: Key,
    pub time: Time,
    pub clef: Clef,
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

#[derive(Debug, Serialize)]
pub struct Note {
    pub pitch: Pitch,
    pub duration: u16,
    // pub r#type: NoteType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Whole,
    Half,
    Quarter,
    Eighth,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct Pitch {
    pub step: Step,
    pub alter: isize,
    pub octave: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Choice {
    Yes,
    No,
}

impl From<bool> for Choice {
    fn from(value: bool) -> Self {
        if value { Choice::Yes } else { Choice::No }
    }
}
