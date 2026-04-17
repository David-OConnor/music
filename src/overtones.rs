//! Relates to mapping notes to frequencies, and sets of overtones.
//! For example, if we choose an even temperament (Traditional), or perfect ratios for
//! a given key. And the overtone series of classes of instruments
//!
//! todo: You may need to split this into separate concepts

use crate::Key;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Temperament {
    /// What most music is; a fixed mapping of frequencies for any key.
    /// Uses simple frequency ratios.
    #[default]
    Even,
    /// Only applicable for a given key. Sounds more *in tune* within that key.
    /// todo: Should thsi be called "Well tempered", or "just intonation?
    WellTempered(Key),
}
