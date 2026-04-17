//! Contains data structures which represent a composition broadly.
//! This does not include more primitive baseline types, but deals with
//! combining them. This is a fuzzy notion, but helps organize the code.

use std::io;

use crate::{
    instrument::Instrument,
    measure::{ChordProgression, Key, Measure, MicroMeasure, SharpFlat},
    note::{Note, NoteLetter},
    overtones::Temperament,
    player,
};
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

/// Likely tentative. Represents all notes which start in a single tick. Will have one note for single notes,
/// multiple notes for coords. This is only the notes which *start* this tick.
pub struct NotesStartingThisTick {
    pub notes: Vec<Note>,
}

impl NotesStartingThisTick {
    pub fn empty() -> Self { Self {notes: Vec::new() } }
}

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
    pub ms_per_tick: u32,
    pub instruments: Vec<Instrument>,
    /// This is indexed by tick, starting at 0.
    pub notes_by_tick: Vec<NotesStartingThisTick>,
    pub measures: Vec<Measure>,
    /// Not required, but may help with generation, improvisation etc.
    pub chord_progression: Option<ChordProgression>,
    /// Default key for notes whose sharp_flat field is None.
    pub key: Key,
    pub temperament: Temperament,
}

impl Composition {
    /// todo: If there are too many params, add a config struct.
    pub fn new(
        // tick_base: TickBase,
        // tick_base: NoteDurationClass,
        ticks_per_sixteenth_note: u32,
        ms_per_tick: u32,
        key: Key,
        temperament: Temperament,
        instruments: Vec<Instrument>,
    ) -> Self {
        Self {
            ticks_per_sixteenth_note,
            ms_per_tick,
            instruments,
            notes_by_tick: Vec::new(),
            measures: Vec::new(),
            chord_progression: None,
            key,
            temperament,
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
