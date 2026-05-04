//! For adding (hopefully) general and flexible classes which can be used to represent
//! percussion, or other notes which are not characterized by pitch. We use the duration
//! mechanisms described elsewhere.

use crate::{
    key_scale::SharpFlat,
    note::{Note, NoteLetter},
};

/// General MIDI (Level 1) Percussion mapping for Channel 10.
/// Ordered by MIDI Note Number (35 to 81).
#[derive(Clone, Copy, PartialEq)]
pub enum PercussionHit {
    AcousticBassDrum, // 35
    Kick,             // 36 (GM: Bass Drum 1)
    SideStick,        // 37
    Snare,            // 38 (GM: Acoustic Snare)
    HandClap,         // 39
    SnareRoll,        // 40 (GM: Electric Snare - used as Roll placeholder)
    Tom5,             // 41 (GM: Low Floor Tom)
    HighhatClosed,    // 42 (GM: Closed Hi-Hat)
    Tom4,             // 43 (GM: High Floor Tom)
    PedalHiHat,       // 44
    Tom3,             // 45 (GM: Low Tom)
    HighhatOpen,      // 46 (GM: Open Hi-Hat)
    Tom2,             // 47 (GM: Low-Mid Tom)
    Tom1,             // 48 (GM: Hi-Mid Tom)
    Crash0,           // 49 (GM: Crash Cymbal 1)
    Tom0,             // 50 (GM: High Tom)
    Ride0,            // 51 (GM: Ride Cymbal 1)
    Gong,             // 52 (GM: Chinese Cymbal - used as Gong placeholder)
    RideBell,         // 53
    Tamborine,        // 54
    SplashCymbal,     // 55
    Cowbell,          // 56
    Crash1,           // 57 (GM: Crash Cymbal 2)
    Vibraslap,        // 58
    Ride1,            // 59 (GM: Ride Cymbal 2)
    HiBongo,          // 60
    LowBongo,         // 61
    MuteHiConga,      // 62
    OpenHiConga,      // 63
    LowConga,         // 64
    HighTimbale,      // 65
    LowTimbale,       // 66
    HighAgogo,        // 67
    LowAgogo,         // 68
    Cabasa,           // 69
    Maracas,          // 70
    ShortWhistle,     // 71
    LongWhistle,      // 72
    ShortGuiro,       // 73
    LongGuiro,        // 74
    Claves,           // 75
    HiWoodBlock,      // 76
    LowWoodBlock,     // 77
    MuteCuica,        // 78
    OpenCuica,        // 79
    MuteTriangle,     // 80
    OpenTriangle,     // 81
}

impl PercussionHit {
    /// Returns the standard General MIDI (Level 1) note mapping for Channel 10.
    /// Assumes the Roland octave convention (MIDI Note 36 = C2).
    pub fn midi_note(self) -> Note {
        use NoteLetter::*;
        use SharpFlat::*;

        match self {
            Self::AcousticBassDrum => Note::new(B, Some(Natural), 1), // 35
            Self::Kick => Note::new(C, Some(Natural), 2),             // 36
            Self::SideStick => Note::new(C, Some(Sharp), 2),          // 37
            Self::Snare => Note::new(D, Some(Natural), 2),            // 38
            Self::HandClap => Note::new(D, Some(Sharp), 2),           // 39
            Self::SnareRoll => Note::new(E, Some(Natural), 2),        // 40
            Self::Tom5 => Note::new(F, Some(Natural), 2),             // 41
            Self::HighhatClosed => Note::new(F, Some(Sharp), 2),      // 42
            Self::Tom4 => Note::new(G, Some(Natural), 2),             // 43
            Self::PedalHiHat => Note::new(G, Some(Sharp), 2),         // 44
            Self::Tom3 => Note::new(A, Some(Natural), 2),             // 45
            Self::HighhatOpen => Note::new(A, Some(Sharp), 2),        // 46
            Self::Tom2 => Note::new(B, Some(Natural), 2),             // 47
            Self::Tom1 => Note::new(C, Some(Natural), 3),             // 48
            Self::Crash0 => Note::new(C, Some(Sharp), 3),             // 49
            Self::Tom0 => Note::new(D, Some(Natural), 3),             // 50
            Self::Ride0 => Note::new(D, Some(Sharp), 3),              // 51
            Self::Gong => Note::new(E, Some(Natural), 3),             // 52
            Self::RideBell => Note::new(F, Some(Natural), 3),         // 53
            Self::Tamborine => Note::new(F, Some(Sharp), 3),          // 54
            Self::SplashCymbal => Note::new(G, Some(Natural), 3),     // 55
            Self::Cowbell => Note::new(G, Some(Sharp), 3),            // 56
            Self::Crash1 => Note::new(A, Some(Natural), 3),           // 57
            Self::Vibraslap => Note::new(A, Some(Sharp), 3),          // 58
            Self::Ride1 => Note::new(B, Some(Natural), 3),            // 59
            Self::HiBongo => Note::new(C, Some(Natural), 4),          // 60
            Self::LowBongo => Note::new(C, Some(Sharp), 4),           // 61
            Self::MuteHiConga => Note::new(D, Some(Natural), 4),      // 62
            Self::OpenHiConga => Note::new(D, Some(Sharp), 4),        // 63
            Self::LowConga => Note::new(E, Some(Natural), 4),         // 64
            Self::HighTimbale => Note::new(F, Some(Natural), 4),      // 65
            Self::LowTimbale => Note::new(F, Some(Sharp), 4),         // 66
            Self::HighAgogo => Note::new(G, Some(Natural), 4),        // 67
            Self::LowAgogo => Note::new(G, Some(Sharp), 4),           // 68
            Self::Cabasa => Note::new(A, Some(Natural), 4),           // 69
            Self::Maracas => Note::new(A, Some(Sharp), 4),            // 70
            Self::ShortWhistle => Note::new(B, Some(Natural), 4),     // 71
            Self::LongWhistle => Note::new(C, Some(Natural), 5),      // 72
            Self::ShortGuiro => Note::new(C, Some(Sharp), 5),         // 73
            Self::LongGuiro => Note::new(D, Some(Natural), 5),        // 74
            Self::Claves => Note::new(D, Some(Sharp), 5),             // 75
            Self::HiWoodBlock => Note::new(E, Some(Natural), 5),      // 76
            Self::LowWoodBlock => Note::new(F, Some(Natural), 5),     // 77
            Self::MuteCuica => Note::new(F, Some(Sharp), 5),          // 78
            Self::OpenCuica => Note::new(G, Some(Natural), 5),        // 79
            Self::MuteTriangle => Note::new(G, Some(Sharp), 5),       // 80
            Self::OpenTriangle => Note::new(A, Some(Natural), 5),     // 81
        }
    }

    /// Returns the raw MIDI integer value (0-127) for this percussion hit.
    /// Highly recommended for writing MIDI events directly to bypass octave-offset ambiguities.
    pub fn midi_number(self) -> u8 {
        self as u8 + 35
    }
}
