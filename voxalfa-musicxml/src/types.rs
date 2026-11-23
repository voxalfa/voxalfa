use serde::Serialize;
use voxalfa_core::data_types::{Dynamic as CoreDynamics, Touch, Voice};

#[derive(Debug, Serialize)]
pub struct TagEnum<T> {
    #[serde(rename = "$value")]
    pub value: T,
}

impl<T> From<T> for TagEnum<T> {
    fn from(value: T) -> Self {
        Self { value }
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScorePartwise {
    #[serde(rename = "@version")]
    pub version: MusicXmlVersion,
    pub credit: Vec<Credit>,
    pub part_list: PartList,
    pub part: Vec<Part>,
}

#[derive(Debug, Default, Serialize)]
pub enum MusicXmlVersion {
    #[serde(rename = "4.0")]
    #[default]
    V4,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Credit {
    #[serde(rename = "@page")]
    pub page: u8,
    pub credit_type: String,
    pub credit_words: String,
}

impl Credit {
    pub fn new(credit_type: &str, credit_words: impl Into<String>) -> Self {
        Self {
            page: 1,
            credit_type: credit_type.to_string(),
            credit_words: credit_words.into(),
        }
    }
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
    Barline(Barline),
}

#[derive(Debug, Serialize)]
pub struct Barline {
    #[serde(rename = "@location")]
    pub location: BarlineLocation,
    pub ending: Ending,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BarlineLocation {
    Left,
    Right,
}

#[derive(Debug, Serialize)]
pub struct Ending {
    #[serde(rename = "@number")]
    pub number: String,
    #[serde(rename = "@type")]
    pub r#type: Position,
    #[serde(rename = "$text")]
    pub content: Option<String>,
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
    pub line: u8,
}

impl Clef {
    pub fn from_voice(voice: Voice) -> Self {
        match voice {
            Voice::S => Self {
                sign: ClefSign::G,
                line: 2,
            },
            Voice::A => Self {
                sign: ClefSign::C,
                line: 3,
            },
            Voice::T => Self {
                sign: ClefSign::C,
                line: 4,
            },
            Voice::B => Self {
                sign: ClefSign::F,
                line: 4,
            },
        }
    }
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
    pub lyric: Vec<Lyric>,
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

#[derive(Debug, Default, Serialize)]
pub struct Notations {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tied: Vec<Tie>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fermata: Option<()>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub articulations: Option<TagEnum<Articulations>>,
}

#[derive(Debug, Serialize)]
pub struct Tie {
    #[serde(rename = "@type")]
    pub r#type: Position,
}

#[derive(Debug, Serialize)]
pub struct Lyric {
    #[serde(rename = "@number")]
    pub number: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syllabic: Option<Syllabic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extend: Option<Extend>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Syllabic {
    Begin,
    End,
    Middle,
    Single,
}

#[derive(Debug, Serialize)]
pub struct Extend {
    #[serde(rename = "@type")]
    pub r#type: Position,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Articulations {
    // Accent,
    Staccato,
    StrongAccent,
}

impl From<Touch> for Articulations {
    fn from(value: Touch) -> Self {
        match value {
            Touch::Staccato => Self::Staccato,
            Touch::Marcato => Self::StrongAccent,
            Touch::Fermata => unreachable!(),
        }
    }
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
#[serde(rename_all = "kebab-case")]
pub struct Direction {
    pub direction_type: Vec<DirectionType>,
    #[serde(rename = "@placement")]
    pub placement: Placement,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<Sound>,
}

impl Direction {
    pub fn jump(label: &str, sound: Sound) -> Self {
        Self {
            placement: Placement::Below,
            direction_type: vec![DirectionType::words(label)],
            sound: Some(sound),
        }
    }

    pub fn mark(direction_type: DirectionType, sound: Sound) -> Self {
        Self {
            placement: Placement::Below,
            direction_type: vec![direction_type],
            sound: Some(sound),
        }
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    #[default]
    Above,
    Below,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct DirectionType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segno: Option<()>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coda: Option<()>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<Words>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamics: Option<TagEnum<Dynamics>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metronome: Option<Metronome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wedge: Option<Wedge>,
}

impl DirectionType {
    pub fn segno() -> Self {
        Self {
            segno: Some(()),
            ..Default::default()
        }
    }

    pub fn coda() -> Self {
        Self {
            coda: Some(()),
            ..Default::default()
        }
    }

    pub fn words(content: &str) -> Self {
        Self {
            words: Some(Words {
                content: content.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn dynamics(value: Dynamics) -> Self {
        Self {
            dynamics: Some(TagEnum { value }),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Dynamics {
    FFF,
    FF,
    F,
    MF,
    MP,
    P,
    PP,
    PPP,
}

impl From<CoreDynamics> for Dynamics {
    fn from(value: CoreDynamics) -> Self {
        match value {
            CoreDynamics::PPP => Self::PPP,
            CoreDynamics::PP => Self::PP,
            CoreDynamics::P => Self::P,
            CoreDynamics::MP => Self::MP,
            CoreDynamics::MF => Self::MF,
            CoreDynamics::F => Self::F,
            CoreDynamics::FF => Self::FF,
            CoreDynamics::FFF => Self::FFF,
            _ => unreachable!("invalid dynamics conversion"),
        }
    }
}

impl Dynamics {
    pub fn velocity(self) -> u16 {
        match self {
            Self::PPP => 16,
            Self::PP => 32,
            Self::P => 48,
            Self::MP => 64,
            Self::MF => 80,
            Self::F => 96,
            Self::FF => 112,
            Self::FFF => 127,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Words {
    #[serde(rename = "$text")]
    pub content: String,
    #[serde(rename = "@font-style", skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Metronome {
    pub beat_unit: NoteType,
    pub per_minute: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Sound {
    #[serde(rename = "@segno", skip_serializing_if = "Option::is_none")]
    pub segno: Option<SoundId>,
    #[serde(rename = "@coda", skip_serializing_if = "Option::is_none")]
    pub coda: Option<SoundId>,
    #[serde(rename = "@tocoda", skip_serializing_if = "Option::is_none")]
    pub tocoda: Option<SoundId>,
    #[serde(rename = "@dacapo", skip_serializing_if = "Option::is_none")]
    pub dacapo: Option<Choice>,
    #[serde(rename = "@fine", skip_serializing_if = "Option::is_none")]
    pub fine: Option<Choice>,
    #[serde(rename = "@dalsegno", skip_serializing_if = "Option::is_none")]
    pub dalsegno: Option<SoundId>,
    #[serde(rename = "@dynamics", skip_serializing_if = "Option::is_none")]
    pub dynamics: Option<u16>,
    #[serde(rename = "@tempo", skip_serializing_if = "Option::is_none")]
    pub tempo: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SoundId {
    Segno,
    Coda,
}

impl Sound {
    pub fn coda(self) -> Self {
        Self {
            coda: Some(SoundId::Coda),
            ..self
        }
    }

    pub fn segno(self) -> Self {
        Self {
            segno: Some(SoundId::Segno),
            ..self
        }
    }

    pub fn fine(self) -> Self {
        Self {
            fine: Some(Choice::Yes),
            ..self
        }
    }

    pub fn tocoda(self) -> Self {
        Self {
            tocoda: Some(SoundId::Coda),
            ..self
        }
    }

    pub fn dalsegno(self) -> Self {
        Self {
            dalsegno: Some(SoundId::Segno),
            ..self
        }
    }

    pub fn dacapo(self) -> Self {
        Self {
            dacapo: Some(Choice::Yes),
            ..self
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Wedge {
    #[serde(rename = "@type")]
    pub r#type: WedgeType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WedgeType {
    Crescendo,
    Diminuendo,
    Stop,
}

#[derive(Debug, Serialize)]
pub enum NoteType {
    #[serde(rename = "whole")]
    Whole,
    #[serde(rename = "half")]
    Half,
    #[serde(rename = "quarter")]
    Quarter,
    #[serde(rename = "eighth")]
    Eighth,
    #[serde(rename = "16th")]
    N16,
    #[serde(rename = "32nd")]
    N32,
    #[serde(rename = "64th")]
    N64,
    #[serde(rename = "128th")]
    N128,
}

impl NoteType {
    pub fn from_factor(factor: usize) -> Self {
        match factor {
            1 => Self::Whole,
            2 => Self::Half,
            4 => Self::Quarter,
            8 => Self::Eighth,
            16 => Self::N16,
            32 => Self::N32,
            64 => Self::N64,
            128 => Self::N128,
            _ => panic!("unsupported note type: {factor}"),
        }
    }
}
