//! Contains data structures which represent a composition broadly.
//! This does not include more primitive baseline types, but deals with
//! combining them. This is a fuzzy notion, but helps organize the code.

use std::io;

use crate::{
    ChordProgression, Key, Note, NoteDuration, NoteDurationClass, TimeSignature,
    instrument::Instrument, player,
};

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

// /// Used for anchoring note durations to discrete time ticks.
// /// If note_class = NoteDuration::Eigth, and tick_time is 200ms, and eigth note is
// /// 200ms, and there can be no sixteenth notes.
// pub struct TickBase {
//     /// This note duration is which note is tick_time in real time units.
//     /// Finest duration in the set.
//     pub note_class: NoteDurationClass,
//     /// ms
//     pub tick_time: u32,
// }

// impl TickBase {
//     pub fn new(note_class: NoteDurationClass, tick_time: u32) -> Self {
//         Self { note_class, tick_time }
//     }
// }

/// A top level structure representing an entire work, with all its details.
/// We are starting using the basics you would use to build a sheet music with,
/// and are expanding it to be more general, so as not to be restricted to traditional
/// western music conventions.
pub struct Composition {
    // pub tick_base: TickBase,
    // pub tick_base: NoteDurationClass,
    /// We use this to scale the NoteDurationClass (16th notes, 8th notes etc) with
    /// the underlying integer tick system. Set to 1 if there is truly no time interval
    /// finer than a 16th note.
    pub ticks_per_sixteenth_note: u32,
    /// This is the base tempo.
    pub ticks_per_s: u32,
    pub instruments: Vec<Instrument>,
    pub notes: Vec<Note>,
    /// Not required, but may help with generation, improvisation etc.
    pub chord_progression: Option<ChordProgression>,
}

impl Composition {
    /// todo: If there are too many params, add a config struct.
    pub fn new(
        // tick_base: TickBase,
        // tick_base: NoteDurationClass,
        ticks_per_sixteenth_note: u32,
        ticks_per_s: u32,
        instruments: Vec<Instrument>,
    ) -> Self {
        Self {
            ticks_per_sixteenth_note,
            ticks_per_s,
            instruments,
            notes: Vec::new(),
            chord_progression: None,
        }
    }

    pub fn add_measure(&mut self, measure: Measure) {}

    pub fn make_micromeasures(&self) -> Vec<MicroMeasure> {
        vec![]
    }

    pub fn make_sheet_music(&self) {
        unimplemented!()
    }

    pub fn play(&self) -> io::Result<()> {
        player::play(self)
    }
}
