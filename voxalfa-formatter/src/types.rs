#[derive(Debug, Clone, Copy)]
pub enum Assignment {
    Metadata,
    InitialParams,
    SectionParams,
}

impl Assignment {
    pub fn prefix(self) -> &'static str {
        match self {
            Assignment::Metadata => "#",
            Assignment::InitialParams | Assignment::SectionParams => "$",
        }
    }

    pub fn rank(self) -> LineRank {
        match self {
            Assignment::Metadata => LineRank::Metadata,
            Assignment::InitialParams => LineRank::InitialParams,
            Assignment::SectionParams => LineRank::SectionParams,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LineRank {
    #[default]
    Fallback,
    LangDirective,
    Metadata,
    InitialParams,
    StyleDirective,
    SectionParams,
    Solfa,
    Lyrics,
    Delimiter,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartialLine {
    pub scope: usize,
    pub rank: LineRank,
    pub line_id: usize,
    pub index: usize,
    pub content: String,
}
