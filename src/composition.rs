//! Contains data structures which represent a composition broadly.
//! This does not include more primitive baseline types, but deals with
//! combining them. This is a fuzzy notion, but helps organize the code.

use std::{fmt, io, path::Path};

use crate::{
    instrument::Instrument,
    key_scale::Key,
    measure::{Measure, MicroMeasure},
    midi, music_xml,
    music_xml::MusicXmlFormat,
    note::NotePlayed,
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
    pub notes: Vec<NotePlayed>,
}

impl NotesStartingThisTick {
    pub fn empty() -> Self {
        Self { notes: Vec::new() }
    }
}

impl fmt::Display for NotesStartingThisTick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.notes.iter().map(|n| n.to_string()).collect();
        write!(f, "[{}]", parts.join(", "))
    }
}

/// A top level structure representing an entire work, with all its details.
/// We are starting using the basics you would use to build a sheet music with,
/// and are expanding it to be more general, so as not to be restricted to traditional
/// western music conventions.
pub struct Composition {
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
    // /// Not required, but may help with generation, improvisation etc.
    // pub chord_progression: Option<ChordProgression>,
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
            // chord_progression: None,
            key,
            temperament,
        }
    }

    /// Safely changes ticks-per-sixteenth note, if able.
    pub fn change_ticks_per_sixteenth_note(&mut self, v: u32) -> io::Result<()> {
        if v > self.ticks_per_sixteenth_note {
            if v.is_multiple_of(self.ticks_per_sixteenth_note) {
                self.ticks_per_sixteenth_note = v;

                // todo: Insert new notes_by_tick in the spacings.

                Ok(())
            } else {
                // todo: If not, check self.notes_by_tick that they can be evenly spaced out.
                // todo: If not, return an error.
                unimplemented!();
            }
        } else {
            // todo: Make sure we can cleanly remove empty spaces between notes. If not, return an error.
            unimplemented!();
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

    pub fn to_musicxml(&self, format: MusicXmlFormat, path: &Path) -> io::Result<()> {
        music_xml::write_musicxml(self, format, path)
    }

    pub fn from_musicxml(path: &Path) -> io::Result<Self> {
        music_xml::read_musicxml(path)
    }

    pub fn to_midi(&self, path: &Path) -> io::Result<()> {
        music_xml::write_midi(self, path)
    }

    pub fn from_midi(path: &Path) -> io::Result<Self> {
        midi::read_midi(path)
    }
}
