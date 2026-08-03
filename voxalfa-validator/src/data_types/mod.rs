mod dynamics;
mod key;
mod navigation;
mod tempo;
mod time_signature;
mod touch;
mod voice;

pub use dynamics::Dynamic;
pub use key::{BaseKey, Key, KeyAccidental};
pub use navigation::{Jump, Mark};
pub use tempo::{ExtendedTempo, ProgressiveTempo, StaticTempo};
pub use time_signature::TimeSignature;
pub use touch::Touch;
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
