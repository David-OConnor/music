//! Contains data structures which represent a composition broadly.
//! This does not include more primitive baseline types, but deals with
//! combining them. This is a fuzzy notion, but helps organize the code.

use std::{fmt, fmt::Display, io, path::Path};

use crate::{
    instrument::Instrument, measure::Measure, midi, music_xml,
    music_xml::MusicXmlFormat, overtones::Temperament, player,
};


#[derive(Default)]
pub struct CompMetadata {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub composer: Option<String>,
    pub copyright: Option<String>,
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
            for voice in &meas.notes {
                note_count += voice.len();
            }
        }

        writeln!(
            f,
            "{}, {} | {} notes | {} measures",
            self.metadata.title,
            self.metadata.subtitle,
            note_count,
            self.measures.len(),
        )?;

        Ok(())
    }
}

impl Composition {
    pub fn new(temperament: Temperament, instruments: Vec<Instrument>) -> Self {
        Self {
            metadata: Default::default(),
            instruments,
            measures: Vec::new(),
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

    pub fn add_measure(&mut self, measure: Measure) {
        self.measures.push(measure);
    }

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
