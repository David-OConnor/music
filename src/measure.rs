//! For dividing a composition into measures.
//! This isn't required to generate a set of notes, but can help with generating, improvisation,
//! analysis etc.

use crate::{
    key_scale::Key,
    note::{Chord},
};

/// A traditional music measure.
#[derive(Clone)]
pub struct Measure {
    // todo: Do we want this? For assigning finer divisions to.
    pub ident: u32,
    pub key: Key,
    pub time_signature: TimeSignature,
    pub tempo: u32,
    pub chord: Option<Chord>,
}

impl Measure {
    pub fn new(key: Key, time_signature: TimeSignature, chord: Option<Chord>, tempo: u32) -> Self {
        Self {
            ident: 0,
            key,
            time_signature,
            chord,
            tempo,
        }
    }
}

impl Measure {
    pub fn to_micro_measure(&self) -> Vec<MicroMeasure> {
        let mut res = Vec::new();

        res
    }
}

/// Describes all actions at the coarsest time granularity which can describe a given instant.
/// It describes everything which is happening at thi state. When used to play a composition,
/// for example, this is what is generated. We compose this from coarser constructs.
pub struct MicroMeasure {
    /// ms. We use the largest
    pub duration: u32,
}

#[derive(Copy, Clone, PartialEq)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl TimeSignature {
    pub fn new(numerator: u8, denominator: u8) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

/// For displaying in sheet music, for example
#[derive(Clone, Copy, PartialEq)]
pub enum Clef {
    Treble,
    Bass,
    Alto,
}

// /// A framework of coords, by measure. This may have more applicability to music generation
// /// than as a fundamental representation of a work.
// pub struct ChordProgression {
//     /// Indexed by measure. These sets can be composed into a broader structure.
//     pub subsets: Vec<Vec<Chord>>,
//     /// (subset index, repetitions)
//     pub sets: Vec<(usize, usize)>,
// }
