#[derive(Debug, Clone, Copy)]
pub enum Assignment {
    Metadata,
    Params,
}

impl Assignment {
    pub fn prefix(self) -> &'static str {
        match self {
            Assignment::Metadata => "#",
            Assignment::Params => "$",
        }
    }

    pub fn rank(self) -> LineRank {
        match self {
            Assignment::Metadata => LineRank::Metadata,
            Assignment::Params => LineRank::Params,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LineRank {
    #[default]
    Fallback,
    Directive,
    Metadata,
    Params,
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
