mod dynamics;
mod key;
mod marker;
mod tempo;
mod time_signature;
mod voice;

// TODO?
// mod articulations;

pub use dynamics::Dynamic;
pub use key::{BaseKey, Key, KeyAccidental};
pub use marker::Navigation;
pub use tempo::Tempo;
pub use time_signature::TimeSignature;
pub use voice::Voice;

use crate::ast::symbols::SymbolRef;

pub type List<T> = Vec<SymbolRef<T>>;
pub type TimedList<T> = List<TimedValue<T>>;

#[derive(Debug, Clone)]
pub struct TimedValue<T> {
    pub start: f32,
    pub end: Option<f32>,
    pub value: T,
}
