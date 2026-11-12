use std::sync::Arc;

use skrifa::{
    FontRef, MetadataProvider,
    prelude::{LocationRef, Size},
};
use voxalfa_core::{
    ast::directives::{ElementStyle, FontStretch},
    output::metrics::StringMetric,
};

use crate::{error::Result, options::RenderOptions};

pub const SOLFA_FONT: &[u8] = include_bytes!("../fonts/FiraSans-Bold.ttf");
pub const TEXT_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");
pub const TEXT_B_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Bold.ttf");
pub const SERIF_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-Regular.ttf");
pub const SERIF_I_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-Italic.ttf");
pub const SERIF_BI_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-BoldItalic.ttf");
pub const LYRIC_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Condensed.ttf");
pub const MUSIC_FONT: &[u8] = include_bytes!("../fonts/NotoSansSymbols-Bold.ttf");

pub const SOLFA_FAMILY: &str = "Fira Sans";
pub const MUSIC_FAMILY: &str = "Noto Sans Symbols";
pub const TEXT_FAMILY: &str = "Noto Sans";
pub const SERIF_FAMILY: &str = "Noto Serif";

const SYMBOL_FONT_SCALE: f32 = 0.85;
const DYNAMIC_FONT_SCALE: f32 = 0.9;

#[derive(Debug, Clone)]
pub struct FontFace {
    pub family: String,
    pub weight: u16,
    pub stretch: FontStretch,
    pub italic: bool,
    pub index: u32,
    pub data: Arc<[u8]>,
}

impl FontFace {
    pub fn new(family: impl Into<String>, data: impl Into<Arc<[u8]>>) -> Self {
        Self {
            family: family.into(),
            weight: 400,
            stretch: FontStretch::Normal,
            italic: false,
            index: 0,
            data: data.into(),
        }
    }

    pub fn with_weight(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }

    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    pub fn with_index(mut self, index: u32) -> Self {
        self.index = index;
        self
    }

    fn same_key(&self, other: &Self) -> bool {
        self.family.eq_ignore_ascii_case(&other.family)
            && self.weight == other.weight
            && self.stretch == other.stretch
            && self.italic == other.italic
            && self.index == other.index
    }

    fn match_score(&self, weight: u16, italic: bool, stretch: FontStretch) -> (u8, u8, u16, u8) {
        let stretch_distance = self.stretch.rank().abs_diff(stretch.rank());
        let style_penalty = u8::from(self.italic != italic);
        let weight_distance = self.weight.abs_diff(weight);
        let lighter_penalty = u8::from(self.weight < weight);

        (
            stretch_distance,
            style_penalty,
            weight_distance,
            lighter_penalty,
        )
    }
}

#[derive(Debug, Clone)]
pub struct FontFaces {
    faces: Vec<FontFace>,
}

impl Default for FontFaces {
    fn default() -> Self {
        Self {
            faces: FaceRole::ALL.into_iter().map(FaceRole::face).collect(),
        }
    }
}

impl FontFaces {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FontFace> {
        self.faces.iter()
    }

    pub fn add(&mut self, face: FontFace) {
        let entry = self
            .faces
            .iter_mut()
            .find(|current| current.same_key(&face));

        if let Some(current) = entry {
            *current = face
        } else {
            self.faces.push(face)
        }
    }

    pub fn with(mut self, face: FontFace) -> Self {
        self.add(face);
        self
    }

    pub fn find(
        &self,
        family: &str,
        weight: u16,
        italic: bool,
        stretch: FontStretch,
    ) -> Option<&FontFace> {
        self.faces
            .iter()
            .filter(|face| face.family.eq_ignore_ascii_case(family))
            .min_by_key(|face| face.match_score(weight, italic, stretch))
    }
}

#[derive(Debug, Clone, Copy)]
struct FaceRole {
    family: &'static str,
    data: &'static [u8],
    weight: u16,
    stretch: FontStretch,
    italic: bool,
}

impl FaceRole {
    const SOLFA: Self = Self::new(SOLFA_FAMILY, SOLFA_FONT).bold();
    const TEXT: Self = Self::new(TEXT_FAMILY, TEXT_FONT);
    const TEXT_B: Self = Self::new(TEXT_FAMILY, TEXT_B_FONT).bold();
    const SERIF: Self = Self::new(SERIF_FAMILY, SERIF_FONT);
    const SERIF_I: Self = Self::new(SERIF_FAMILY, SERIF_I_FONT).italic();
    const SERIF_BI: Self = Self::new(SERIF_FAMILY, SERIF_BI_FONT).bold().italic();
    const LYRIC: Self = Self::new(TEXT_FAMILY, LYRIC_FONT).stretch(FontStretch::Condensed);
    const MUSIC: Self = Self::new(MUSIC_FAMILY, MUSIC_FONT).bold();

    const ALL: [Self; 8] = [
        Self::SOLFA,
        Self::TEXT,
        Self::TEXT_B,
        Self::SERIF,
        Self::SERIF_I,
        Self::SERIF_BI,
        Self::LYRIC,
        Self::MUSIC,
    ];

    const fn new(family: &'static str, data: &'static [u8]) -> Self {
        Self {
            family,
            data,
            weight: 400,
            stretch: FontStretch::Normal,
            italic: false,
        }
    }

    const fn bold(mut self) -> Self {
        self.weight = 700;
        self
    }

    const fn stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }

    const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    fn for_element(element: &str) -> Option<Self> {
        let role = match element {
            "note-text" | "octave-text" | "divider-text" => Self::SOLFA,
            "lyrics-text" => Self::LYRIC,
            "title-text" | "key-text" | "key-label" | "jump-label" | "volta-label" => Self::TEXT_B,
            "author-text" | "composer-text" | "time-text" | "meter-text" | "footer-text"
            | "time-label" => Self::TEXT,
            "mark-label" | "tempo-label" => Self::SERIF_I,
            "bpm-label" => Self::SERIF,
            "dynamic-label" => Self::SERIF_BI,
            _ => return None,
        };

        Some(role)
    }

    fn face(self) -> FontFace {
        FontFace::new(self.family, self.data)
            .with_weight(self.weight)
            .with_stretch(self.stretch)
            .with_italic(self.italic)
    }

    fn query(self) -> FaceQuery<'static> {
        FaceQuery {
            family: self.family,
            weight: self.weight,
            stretch: self.stretch,
            italic: self.italic,
        }
    }
}

pub fn default_font_style(element: &str) -> Option<(u16, bool, FontStretch)> {
    FaceRole::for_element(element).map(|role| (role.weight, role.italic, role.stretch))
}

#[derive(Debug, Clone, Copy)]
struct FaceQuery<'a> {
    family: &'a str,
    weight: u16,
    stretch: FontStretch,
    italic: bool,
}

impl<'a> FaceQuery<'a> {
    fn for_style(role: FaceRole, style: &'a ElementStyle) -> Self {
        Self {
            family: style.font_family.as_deref().unwrap_or(role.family),
            weight: style.font_weight.unwrap_or(role.weight),
            stretch: style.font_stretch.unwrap_or(role.stretch),
            italic: style.italic.unwrap_or(role.italic),
        }
    }
}

pub struct FontMetrics {
    face: FontFace,
    weight: u16,
    stretch: FontStretch,
    italic: bool,
    size: f32,
    ascent: f32,
    descent: f32,
}

impl FontMetrics {
    fn new(
        face: FontFace,
        weight: u16,
        stretch: FontStretch,
        italic: bool,
        size: f32,
    ) -> Result<Self> {
        let font = FontRef::from_index(&face.data, face.index)?;
        let metrics = font.metrics(Size::new(size), LocationRef::default());

        Ok(Self {
            face,
            weight,
            stretch,
            italic,
            size,
            ascent: metrics.ascent,
            descent: metrics.descent.abs(),
        })
    }

    pub fn family(&self) -> &str {
        &self.face.family
    }

    pub fn weight(&self) -> u16 {
        self.weight
    }

    pub fn stretch(&self) -> FontStretch {
        self.stretch
    }

    pub fn italic(&self) -> bool {
        self.italic
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    pub fn descent(&self) -> f32 {
        self.descent
    }

    pub fn line_height(&self) -> f32 {
        self.ascent + self.descent
    }

    pub fn width(&self, text: &str) -> f32 {
        let Ok(font) = FontRef::from_index(&self.face.data, self.face.index) else {
            return 0.0;
        };

        let glyph_metrics = font.glyph_metrics(Size::new(self.size), LocationRef::default());
        let charmap = font.charmap();

        text.chars()
            .filter_map(|ch| charmap.map(ch))
            .filter_map(|id| glyph_metrics.advance_width(id))
            .sum()
    }
}

impl StringMetric for &FontMetrics {
    type Output = f32;

    fn measure_string(&self, text: &str) -> Self::Output {
        self.width(text)
    }
}

pub struct Fonts {
    pub text: TextFonts,
    pub label: LabelFonts,
    pub touch: FontMetrics,
    pub faces: Vec<FontFace>,
}

pub struct TextFonts {
    pub note: FontMetrics,
    pub octave: FontMetrics,
    pub lyric: FontMetrics,
    pub title: FontMetrics,
    pub author: FontMetrics,
    pub composer: FontMetrics,
    pub key: FontMetrics,
    pub time: FontMetrics,
    pub meter: FontMetrics,
    pub footer: FontMetrics,
    pub divider: FontMetrics,
}

pub struct LabelFonts {
    pub key: FontMetrics,
    pub jump: FontMetrics,
    pub mark: FontMetrics,
    pub dynamic: FontMetrics,
    pub tempo: FontMetrics,
    pub bpm: FontMetrics,
    pub time: FontMetrics,
    pub volta: FontMetrics,
}

impl Fonts {
    pub fn with_faces(options: &RenderOptions, faces: &FontFaces) -> Result<Self> {
        let fallback = FontFaces::default();

        let mut builder = FontBuilder::new(faces, &fallback);

        let text = TextFonts::new(options, &mut builder)?;
        let label = LabelFonts::new(options, &mut builder)?;

        let touch = builder.metric(
            FaceRole::SERIF,
            options.marker_font_size,
            &ElementStyle::default(),
        )?;

        let music = builder.resolve(FaceRole::MUSIC, FaceRole::MUSIC.query());

        builder.register(&music);

        Ok(Self {
            text,
            label,
            touch,
            faces: builder.resolved,
        })
    }
}

impl TextFonts {
    fn new(opts: &RenderOptions, builder: &mut FontBuilder<'_>) -> Result<Self> {
        let styles = &opts.elements.text;
        let meta = opts.meta_font_size;

        Ok(Self {
            note: builder.metric(FaceRole::SOLFA, opts.solfa_font_size, &styles.note)?,
            octave: builder.metric(FaceRole::SOLFA, opts.octave_font_size, &styles.octave)?,
            lyric: builder.metric(FaceRole::LYRIC, opts.lyric_font_size, &styles.lyrics)?,
            title: builder.metric(FaceRole::TEXT_B, opts.title_font_size, &styles.title)?,
            author: builder.metric(FaceRole::TEXT, opts.name_font_size, &styles.author)?,
            composer: builder.metric(FaceRole::TEXT, opts.name_font_size, &styles.composer)?,
            key: builder.metric(FaceRole::TEXT_B, opts.marker_font_size, &styles.key)?,
            time: builder.metric(FaceRole::TEXT, opts.marker_font_size, &styles.time)?,
            meter: builder.metric(FaceRole::TEXT, meta, &styles.meter)?,
            footer: builder.metric(FaceRole::TEXT, opts.footer_font_size, &styles.footer)?,
            divider: builder.metric(
                FaceRole::SOLFA,
                opts.solfa_font_size * SYMBOL_FONT_SCALE,
                &styles.divider,
            )?,
        })
    }
}

impl LabelFonts {
    fn new(options: &RenderOptions, builder: &mut FontBuilder<'_>) -> Result<Self> {
        let styles = &options.elements.label;
        let marker = options.marker_font_size;

        Ok(Self {
            key: builder.metric(FaceRole::TEXT_B, marker, &styles.key)?,
            jump: builder.metric(FaceRole::TEXT_B, marker, &styles.jump)?,
            mark: builder.metric(FaceRole::SERIF_I, marker, &styles.mark)?,
            dynamic: builder.metric(
                FaceRole::SERIF_BI,
                marker * DYNAMIC_FONT_SCALE,
                &styles.dynamic,
            )?,
            tempo: builder.metric(FaceRole::SERIF_I, marker, &styles.tempo)?,
            bpm: builder.metric(FaceRole::SERIF, marker, &styles.bpm)?,
            time: builder.metric(FaceRole::TEXT, marker, &styles.time)?,
            volta: builder.metric(FaceRole::TEXT_B, marker, &styles.volta)?,
        })
    }
}

struct FontBuilder<'a> {
    faces: &'a FontFaces,
    fallback: &'a FontFaces,
    resolved: Vec<FontFace>,
}

impl<'a> FontBuilder<'a> {
    fn new(faces: &'a FontFaces, fallback: &'a FontFaces) -> Self {
        Self {
            faces,
            fallback,
            resolved: Vec::new(),
        }
    }

    fn metric(
        &mut self,
        role: FaceRole,
        default_size: f32,
        style: &ElementStyle,
    ) -> Result<FontMetrics> {
        let query = FaceQuery::for_style(role, style);

        let mut face = self.resolve(role, query);

        if !face.family.eq_ignore_ascii_case(query.family) {
            face.family = query.family.to_string();
        }

        let metrics = FontMetrics::new(
            face.clone(),
            query.weight,
            query.stretch,
            query.italic,
            style.size(default_size),
        )?;

        self.register(&face);

        Ok(metrics)
    }

    fn resolve(&self, role: FaceRole, query: FaceQuery<'_>) -> FontFace {
        self.faces
            .find(query.family, query.weight, query.italic, query.stretch)
            .or_else(|| {
                self.fallback
                    .find(query.family, query.weight, query.italic, query.stretch)
            })
            .or_else(|| {
                self.fallback
                    .find(role.family, role.weight, role.italic, role.stretch)
            })
            .cloned()
            .unwrap_or_else(|| role.face())
    }

    fn register(&mut self, face: &FontFace) {
        if !self.resolved.iter().any(|current| current.same_key(face)) {
            self.resolved.push(face.clone());
        }
    }
}
