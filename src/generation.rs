//! Generation of music, perhaps based on NNs. (Burn?)

use egui::Key;
use musicxml::elements::Chord;

use crate::{
    composition::Composition, composition_arch::CompositionComponent, measure::TimeSignature,
    rhythm::RhythmPattern,
};

pub struct CompositionGuide {
    // todo: Key and time sig can change; placeholder for now.
    pub key: Key,
    pub time_sig: TimeSignature,
    pub chords: Vec<Chord>,

    pub rhythm_pattern: RhythmPattern,
    pub comps: Vec<CompositionComponent>,
}

impl CompositionGuide {
    pub fn make_comp(&self) -> Composition {
        todo!()
    }
}
