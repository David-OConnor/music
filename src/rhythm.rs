//! Used for generating music. Describes the rhythmic feel of a  measure, composition, etc.
//! Can define primary beats (Where for example multiple instruments/voices are expected to play
//! a note in sync in a given measure or set of measures) This may define where syncopation occurs,
//! and is generally used to define rhythmic patterns which unite voices and instruments

use std::{io, path::Path};

use crate::measure::TimeSignature;

/// For now, this is for a given measure.
#[derive(Clone, Default)]
pub struct RhythmPattern {
    /// todo: Duration ticks on a relative scale? In a given measure?
    // /// For example, the kick drum and bass instrument may be expected to both strike a notable
    // /// note together on the first and third beat of each measure.
    // pub hits_primary: Vec<u32>,
    // pub hits_secondary: Vec<u32>,
    // pub hits_tertiary: Vec<u32>,
    /// These are as (measure division, which of these divisions).
    /// For example (These examples are all for 4/4, but this interface is generalizable)
    ///   - (4, vec![1]) : A beat on the first quarter note
    ///   - (4, vec![1, 3]) : A beat on the first and third quarter notes
    ///   - (8, vec![0, 2, 4, 6, 7]) : A beat every other 8th note, plus the final one.
    /// todo: Make a single vec instead of hard-coded prim/sec/ter indivdidual fields, if that makes more sense..
    pub hits_primary: (u8, Vec<u8>),
    pub hits_secondary: (u8, Vec<u8>),
    pub hits_tertiary: (u8, Vec<u8>),
}

impl RhythmPattern {
    /// Preset. Primary beat on each measure's downbeat. Secondary on every other main note
    /// as defined by the time signature's denominator.
    pub fn measure_downbeats(sig: TimeSignature) -> Self {
        let secondary: Vec<_> = (0..sig.numerator).collect();

        Self {
            hits_primary: (sig.numerator, vec![0]),
            hits_secondary: (sig.numerator, secondary),
            ..Default::default()
        }
    }

    /// Preset.
    pub fn syncopated(sig: TimeSignature) -> Self {
        // todo: QC this fn.
        let mut primary = vec![0];
        if sig.numerator.is_multiple_of(2) {
            primary.push(sig.numerator / 2);
        }

        let secondary: Vec<_> = (1..sig.numerator * 2).collect();

        Self {
            hits_primary: (sig.numerator, primary),
            hits_secondary: (sig.numerator * 2, secondary),
            ..Default::default()
        }
    }

    /// todo: A/R
    /// todo: Make bass guitar or piano LH too?
    pub fn make_midi_drums(&self, path: &Path) -> io::Result<()> {
        Ok(())
    }
}
