//! Contains data structures which represent a composition broadly.
//! This does not include more primitive baseline types, but deals with
//! combining them. This is a fuzzy notion, but helps organize the code.

use std::{fmt, fmt::Display, io, path::Path};

use crate::{
    instrument::Instrument, key_scale::Key, measure::Measure, midi, music_xml,
    music_xml::MusicXmlFormat, note::NotePlayed, overtones::Temperament, player,
};

/// Represents all notes which start in a single tick. Will have one note for single notes,
/// multiple notes for coords. This is only the notes which *start* this tick.
pub struct NotesStartingThisTick {
    pub notes: Vec<NotePlayed>,
}

impl NotesStartingThisTick {
    pub fn empty() -> Self {
        Self { notes: Vec::new() }
    }
}

impl Display for NotesStartingThisTick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.notes.iter().map(|n| n.to_string()).collect();
        write!(f, "[{}]", parts.join(", "))
    }
}

// /// We divide a composition into these: Each has its own note precision, and total duration.
// /// This allows areas of high note density to have a fine tick precision, while sections of whole
// /// notes, for example, to be coarser.
// ///
// /// These are arranged linearly in a composition.
// /// This may be used as a measure, or not.
// pub struct NoteSet {
//     /// Number of divisions in this set. Higher means more precise.
//     /// 12, 16, and 32 are convenient defaults.
//     pub divisions: u16,
//     /// Time of this set, in ms.
//     pub duration: u32, // todo: time in ms?
//     // /// We use this to scale the NoteDurationClass (16th notes, 8th notes etc) with
//     // /// the underlying integer tick system. Set to 1 if there is truly no time interval
//     // /// finer than a 16th note.
//     // pub ticks_per_sixteenth_note: u32,
//     // /// This is the base tempo.
//     // pub ms_per_tick: u32,
// }

#[derive(Default)]
pub struct CompMetadata {
    pub title: String,
    pub subtitle: String,
    pub composer: String,
}

/// A top level structure representing an entire work, with all its details.
/// We are starting using the basics you would use to build a sheet music with,
/// and are expanding it to be more general, so as not to be restricted to traditional
/// western music conventions.
pub struct Composition {
    metadata: CompMetadata,
    // /// We use this to scale the NoteDurationClass (16th notes, 8th notes etc) with
    // /// the underlying integer tick system. Set to 1 if there is truly no time interval
    // /// finer than a 16th note.
    // pub ticks_per_sixteenth_note: u32,
    // /// This is the base tempo.
    // pub ms_per_tick: u32,
    pub instruments: Vec<Instrument>,
    /// This is indexed by tick, starting at 0.
    // pub notes_by_tick: Vec<NotesStartingThisTick>,
    // pub note_sets: Vec<NoteSet>,
    pub measures: Vec<Measure>,
    // /// Not required, but may help with generation, improvisation etc.
    // pub chord_progression: Option<ChordProgression>,
    /// Default key for notes whose sharp_flat field is None. todo: Remove?
    // pub key: Key,
    pub temperament: Temperament,
}

impl Display for Composition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut note_count = 0;
        for meas in &self.measures {
            note_count += meas.notes.len();
        }

        writeln!(
            f,
            "{}, {} - {} | {} notes | {} measures",
            self.metadata.title,
            self.metadata.subtitle,
            self.key,
            // self..len(),
            note_count,
            self.measures.len(),
            // self.ms_per_tick,
            // self.ticks_per_sixteenth_note
        )?;

        Ok(())
    }
}

impl Composition {
    /// todo: If there are too many params, add a config struct.
    pub fn new(
        // tick_base: TickBase,
        // tick_base: NoteDurationClass,
        // ticks_per_sixteenth_note: u32,
        // ms_per_tick: u32,
        key: Key,
        temperament: Temperament,
        instruments: Vec<Instrument>,
    ) -> Self {
        Self {
            metadata: Default::default(),
            // ticks_per_sixteenth_note,
            // ms_per_tick,
            instruments,
            // notes_by_tick: Vec::new(),
            measures: Vec::new(),
            // chord_progression: None,
            key,
            temperament,
        }
    }

    // /// Safely changes ticks-per-sixteenth note, if able.
    // pub fn change_ticks_per_sixteenth_note(&mut self, v: u32) -> io::Result<()> {
    //     if v > self.ticks_per_sixteenth_note {
    //         if v.is_multiple_of(self.ticks_per_sixteenth_note) {
    //             self.ticks_per_sixteenth_note = v;
    //
    //             // todo: Insert new notes_by_tick in the spacings.
    //
    //             Ok(())
    //         } else {
    //             // todo: If not, check self.notes_by_tick that they can be evenly spaced out.
    //             // todo: If not, return an error.
    //             unimplemented!();
    //         }
    //     } else {
    //         // todo: Make sure we can cleanly remove empty spaces between notes. If not, return an error.
    //         unimplemented!();
    //     }
    // }

    pub fn add_measure(&mut self, measure: Measure) {}

    pub fn play(&self) -> io::Result<()> {
        player::play(self)
    }

    pub fn save_musicxml(&self, format: MusicXmlFormat, path: &Path) -> io::Result<()> {
        music_xml::write_musicxml(self, format, path)
    }

    pub fn load_musicxml(path: &Path) -> io::Result<Self> {
        music_xml::read_musicxml(path)
    }

    pub fn save_midi(&self, path: &Path) -> io::Result<()> {
        midi::write_midi(self, path)
    }

    pub fn load_midi(path: &Path) -> io::Result<Self> {
        midi::read_midi(path)
    }
}
