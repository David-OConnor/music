//! Used for generating music. Describes the rhythmic feel of a  measure, composition, etc.
//! Can define primary beats (Where for example multiple instruments/voices are expected to play
//! a note in sync in a given measure or set of measures) This may define where syncopation occurs,
//! and is generally used to define rhythmic patterns which unite voices and instruments

use std::collections::BTreeMap;

use crate::measure::TimeSignature;

/// Relative emphasis of a rhythm hit. Used by generators to map hits to instrument voices,
/// e.g. primary -> kick / chord, secondary -> snare / passing bass note, tertiary -> hi-hat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitPriority {
    Primary,
    Secondary,
    Tertiary,
}

/// For now, this is for a given measure. Primary hits are the most rhythmically pronounced. Perhaps,
/// a bass drum and bass guitar. Secondary hits are perhaps less proncounced, and so on.
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

    // /// todo: A/R
    // /// todo: Make bass guitar or piano LH too?
    // pub fn make_midi_drums(&self, path: &Path) -> io::Result<()> {
    //     Ok(())
    // }

    /// Convert one hit (`idx` of `division` subdivisions of a measure) to a tick within
    /// a measure of `total_ticks`.
    pub fn hit_to_tick(division: u8, idx: u8, total_ticks: u32) -> u32 {
        if division == 0 {
            return 0;
        }
        (idx as u32 * total_ticks) / division as u32
    }

    /// Sorted, deduped tick positions for primary hits in a measure of `total_ticks` ticks.
    pub fn primary_ticks(&self, total_ticks: u32) -> Vec<u32> {
        Self::hit_ticks(&self.hits_primary, total_ticks)
    }

    pub fn secondary_ticks(&self, total_ticks: u32) -> Vec<u32> {
        Self::hit_ticks(&self.hits_secondary, total_ticks)
    }

    pub fn tertiary_ticks(&self, total_ticks: u32) -> Vec<u32> {
        Self::hit_ticks(&self.hits_tertiary, total_ticks)
    }

    fn hit_ticks(spec: &(u8, Vec<u8>), total_ticks: u32) -> Vec<u32> {
        let (div, hits) = (spec.0, &spec.1);
        if div == 0 {
            return Vec::new();
        }
        let mut v: Vec<u32> = hits
            .iter()
            .map(|&i| Self::hit_to_tick(div, i, total_ticks))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// All hits across the three priorities, deduped by tick (highest priority wins ties)
    /// and sorted ascending by tick position.
    pub fn all_hits(&self, total_ticks: u32) -> Vec<(u32, HitPriority)> {
        let mut map: BTreeMap<u32, HitPriority> = BTreeMap::new();
        for t in self.tertiary_ticks(total_ticks) {
            map.insert(t, HitPriority::Tertiary);
        }
        for t in self.secondary_ticks(total_ticks) {
            map.insert(t, HitPriority::Secondary);
        }
        for t in self.primary_ticks(total_ticks) {
            map.insert(t, HitPriority::Primary);
        }
        map.into_iter().collect()
    }
}
