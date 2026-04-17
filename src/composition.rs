//! Contains data structures which represent a composition broadly.
//! This does not include more primitive baseline types, but deals with
//! combining them. This is a fuzzy notion, but helps organize the code.

use std::io;

use crate::{player, Key, Note, NoteDurationClass, TimeSignature};
use crate::instrument::Instrument;

/// A traditional music measure.
#[derive(Clone)]
pub struct Measure {
    // todo: Do we want this? For assigning finer divisions to.
    pub ident: u32,
    pub key: Key,
    pub time_signature: TimeSignature,
    pub tempo: u32,
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

/// A top level structure representing an entire work, with all its details.
/// We are starting using the basics you would use to build a sheet music with,
/// and are expanding it to be more general, so as not to be restricted to traditional
/// western music conventions.
pub struct Composition {
    /// The base time here is used to set the tempo of the composition.
    /// The specifics of how this is set don't restrict the piece, and are changeable.
    /// This is used as a relative grounding.
    /// todo: You may wish to decouple this from the notation notes.
    pub base_time_duration_class: NoteDurationClass,
    /// Duration of the base class in ms.
    pub base_time_unit: u32,
    pub instruments: Vec<Instrument>,
    pub note: Vec<Note>,
}

impl Composition {
    /// todo: If there are too many params, add a config struct.
    pub fn new(
        base_time_duration_class: NoteDurationClass,
        base_time_unit: u32,
        instruments: Vec<Instrument>
    ) -> Self {

        Self {
            base_time_duration_class,
            base_time_unit,
            instruments,
            note: Vec::new()
        }
    }

    pub fn add_measure(&mut self, measure: Measure) {

    }

    pub fn make_micromeasures(&self) -> Vec<MicroMeasure> {
        vec![]
    }

    pub fn play(&self) -> io::Result<()> {
        player::play(self)
    }
}

