//! For dividing a composition into measures.

use crate::{chord::Chord, key_scale::Key, note::NotePlayed};

/// A traditional music measure. We also use this as a fundamental part of how we
/// store notes in compositions, and managing subdividing compositions into integer
/// time ticks. This somewhat follows how MusicXML defines measures, but only loosely
#[derive(Clone)]
pub struct Measure {
    pub key: Key,
    pub time_signature: TimeSignature,
    /// Beats per minute
    pub tempo: u16,
    pub chord: Option<Chord>,
    /// Index determines visual position if displaying sheet music; top to bottom.
    pub staves: Vec<Staff>,
    // /// Used to deconflict notes for displaying on sheet music. Notably comes up on
    // /// piano and other multi-note instruments. Similar to the implementation in MusicXml.
    // pub num_voices: usize,
    /// Number of divisions in this set. Higher means more precise.
    /// 12, 16, and 32 are convenient defaults. This is the same concept as divisions in
    /// MusicXml.
    ///
    /// Note: MIDI uses the concept of "pulses per quarter note". 96 is a historical default,
    /// and DAWS may default to 960. Higher means more precision.
    pub divisions: u16,
    /// Outer: Voices. Inner: Notes in that voice. Voice indices must stay consistent
    /// throughout the entire composition.
    ///
    /// Voices are used to deconflict notes for displaying on sheet music. Notably comes up on
    /// piano and other multi-note instruments. Similar to the implementation in MusicXml.
    pub notes: Vec<Vec<Vec<NotePlayed>>>,
}

impl Measure {
    pub fn new(key: Key, time_signature: TimeSignature, chord: Option<Chord>, tempo: u16) -> Self {
        Self {
            key,
            time_signature,
            chord,
            tempo,
            staves: vec![Staff::Grand],
            // num_voices: 0,
            divisions: 32,
            notes: Vec::new(),
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl TimeSignature {
    pub fn new(numerator: u8, denominator: u8) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

/// For displaying in sheet music, for example.
/// todo: Call it Clef?
#[derive(Clone, Copy, PartialEq)]
pub enum Staff {
    Treble,
    Bass,
    Alto,
    /// Treble and bass
    Grand,
    Tenor,
    Soprano,
    MezzoSoprano,
    Baritone,
    Subbass,
}
