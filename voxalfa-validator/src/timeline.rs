use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub offset: usize,
    pub duration: usize,
    pub factor: usize,
}

#[derive(Debug, Default)]
pub struct Timeline {
    pub dynamics: BTreeMap<Timestamp, DynamicEvent>,
}

#[derive(Debug)]
pub enum DynamicEvent {
    Mark(MarkDynamic),
    Start(RangeDynamic),
    End(RangeDynamic),
}

#[derive(Debug)]
pub enum MarkDynamic {
    P,
    MP,
    PP,
    PPP,
    F,
    MF,
    FF,
    FFF,
    DC,
    DS,
    Seg,
}

#[derive(Debug)]
pub enum RangeDynamic {
    Cre,
    Dec,
}
