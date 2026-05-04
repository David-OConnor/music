//! Generation of music, perhaps based on NNs. (Burn?)

use egui::Key;
use musicxml::elements::Chord;

use crate::{
    composition::Composition, composition_arch::CompositionComponent, measure::TimeSignature,
    rhythm::RhythmPattern,
};

/// We use this to create a simple composition from a structure. Example use: Specify a key, time
/// signature, chords and rhythm pattern.
pub struct CompositionGuide {
    // todo: Key and time sig can change; placeholder for now.
    // todo: Likely: Break this into sections; perhaps a Vec of this struct instead of
    // todo something new.
    pub key: Key,
    pub time_sig: TimeSignature,
    /// By measure.
    pub chords: Vec<Chord>,
    /// By measure. Must match chord len.
    pub rhythm_pattern: RhythmPattern,
    // todo: A/R. Not implemented yet.
    pub comps: Vec<CompositionComponent>,
}

impl CompositionGuide {
    /// Makes a composition with the following (Hard-coded for now) instruments:
    /// - Drums: Plays the most common drum hits, following all rhythm hits, prioritrizing primary,
    /// then secondary, then tertiary. Might use kick, snare, ride, toms as primaries, and accent
    /// with crash etc.
    /// - Piano: Plays simple chord structures, roughly following the primary rhythm beats
    /// - Bass guitar: plays arbitrary notes from the chords, following the primary and seconary
    /// rhythm beats
    pub fn make_comp(&self) -> Composition {
        todo!()
    }
}
