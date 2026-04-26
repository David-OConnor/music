//! Contains data structures which represent a composition broadly.
//! This does not include more primitive baseline types, but deals with
//! combining them. This is a fuzzy notion, but helps organize the code.

use std::{fmt, fmt::Display, io, path::Path};

use crate::{
    instrument::Instrument, measure::Measure, midi, music_xml, music_xml::MusicXmlFormat,
    overtones::Temperament, player,
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
    pub metadata: CompMetadata,
    /// Measures contain notes and other data in convenient divisions. Outer: Parts. (E.g. different
    /// parts played by each instrument). Inner: Measures for that part.
    pub measures_by_part: Vec<(Instrument, Vec<Measure>)>,
    /// Default key for notes whose sharp_flat field is None. todo: Remove?
    pub temperament: Temperament,
}

impl Display for Composition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut note_count = 0;
        let mut measure_count = 0;
        for (_, measures) in &self.measures_by_part {
            measure_count += measures.len();
            for measure in measures {
                for voice in &measure.notes {
                    note_count += voice.len();
                }
            }
        }

        let title = self.metadata.title.as_deref().unwrap_or("Untitled");
        let subtitle = self.metadata.subtitle.as_deref().unwrap_or("-");
        writeln!(
            f,
            "{}, {} | {} notes | {} measures",
            title, subtitle, note_count, measure_count,
        )?;

        Ok(())
    }
}

impl Composition {
    pub fn new(temperament: Temperament, instruments: Vec<Instrument>) -> Self {
        Self {
            metadata: Default::default(),
            measures_by_part: instruments
                .into_iter()
                .map(|instr| (instr, Vec::new()))
                .collect(),
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

    pub fn add_measure(&mut self, instrument: Instrument, measure: Measure) {
        if let Some((_, measures)) = self
            .measures_by_part
            .iter_mut()
            .find(|(part_instrument, _)| *part_instrument == instrument)
        {
            measures.push(measure);
        } else {
            self.measures_by_part.push((instrument, vec![measure]));
        }
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
