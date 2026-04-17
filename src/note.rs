//! Notes, chords etc. This is a relatively primitive set of types used in both
//! playback, and generation.

use std::io;

use crate::{
    measure::{Key, SharpFlat},
    overtones::Temperament,
};

/// For representing notes in sheet music, for example. Internally, we use integers to
/// represent durations.
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum NoteDurationClass {
    Whole = 1,
    Half = 2,
    Quarter = 4,
    Eighth = 8,
    Sixteenth = 16,
    ThirtySecond = 32,
    SixtyFourth = 64,
    OneTwentyEighth = 128,
}

/// All integer times are in ms. All frequencies are in Hz.
#[derive(Clone, Copy, PartialEq)]
pub enum NoteLetter {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

#[derive(Clone)]
pub struct Note {
    pub letter: NoteLetter,
    /// If none, rever to the key.
    pub sharp_flat: Option<SharpFlat>,
    pub octave: u8,
    pub duration: NoteDuration,
    pub amplitude: f32,
}

impl Note {
    pub fn frequency(&self, key: Key, temperament: Temperament) -> f32 {
        use NoteLetter::*;

        use crate::measure::SharpFlat::*;

        let sf = match self.sharp_flat {
            Some(sf) => sf,
            None => {
                let ks = key.get_sharps_flats();
                match self.letter {
                    A => ks.a,
                    B => ks.b,
                    C => ks.c,
                    D => ks.d,
                    E => ks.e,
                    F => ks.f,
                    G => ks.g,
                }
            }
        };

        let semitone_in_octave = match self.letter {
            C => 0,
            D => 2,
            E => 4,
            F => 5,
            G => 7,
            A => 9,
            B => 11,
        } + match sf {
            Sharp => 1,
            Flat => -1,
            Natural => 0,
        };

        // MIDI note: C4 = 60, A4 = 69
        let midi = (self.octave as i32 + 1) * 12 + semitone_in_octave;

        match temperament {
            Temperament::Even => 440.0 * 2f32.powf((midi - 69) as f32 / 12.0),
            Temperament::WellTempered(wt_key) => {
                let tonic_pc = match wt_key.base_note {
                    C => 0i32,
                    D => 2,
                    E => 4,
                    F => 5,
                    G => 7,
                    A => 9,
                    B => 11,
                } + match wt_key.sharp_flat {
                    Sharp => 1,
                    Flat => -1,
                    Natural => 0,
                };

                let interval = (midi - tonic_pc).rem_euclid(12) as usize;
                let tonic_midi = midi - interval as i32;
                let tonic_freq = 440.0 * 2f32.powf((tonic_midi - 69) as f32 / 12.0);

                // Just intonation ratios from tonic
                let ratio: f32 = [
                    1.0,         // P1 unison
                    16.0 / 15.0, // m2
                    9.0 / 8.0,   // M2
                    6.0 / 5.0,   // m3
                    5.0 / 4.0,   // M3
                    4.0 / 3.0,   // P4
                    45.0 / 32.0, // TT tritone
                    3.0 / 2.0,   // P5
                    8.0 / 5.0,   // m6
                    5.0 / 3.0,   // M6
                    9.0 / 5.0,   // m7
                    15.0 / 8.0,  // M7
                ][interval];

                tonic_freq * ratio
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ChordType {
    Major,
    Minor,
}

/// For now, we are not including suspensions: Use a-la-carte Vec<Note> for that.
#[derive(Clone, Copy, PartialEq)]
pub enum ChordAugmentation {
    Diminished,
    Augmented,
}

/// Presets which follow music conventions. Used to construct more general
/// structures.
pub struct Chord {
    pub base: NoteLetter,
    pub chord_type: ChordType,
    pub augmentation: Option<ChordAugmentation>,
    pub octave: u8,
    pub duration: NoteDuration,
    pub amplitude: f32,
}

impl Chord {
    pub fn notes(&self) -> Vec<Note> {
        use ChordAugmentation::*;
        use ChordType::*;
        use NoteLetter::*;
        use SharpFlat::*;

        // Intervals in semitones from root: [root, third, fifth]
        let intervals: [i32; 3] = match (self.chord_type, self.augmentation) {
            (Major, None) => [0, 4, 7],
            (Minor, None) => [0, 3, 7],
            (Major, Some(Augmented)) => [0, 4, 8],
            (Minor, Some(Diminished)) => [0, 3, 6],
            (Major, Some(Diminished)) => [0, 4, 6],
            (Minor, Some(Augmented)) => [0, 3, 8],
        };

        let base_semitone = match self.base {
            C => 0,
            D => 2,
            E => 4,
            F => 5,
            G => 7,
            A => 9,
            B => 11,
        };

        intervals
            .iter()
            .map(|&offset| {
                let abs = base_semitone + offset;
                let (letter, sharp_flat) = match abs.rem_euclid(12) {
                    0 => (C, Natural),
                    1 => (C, Sharp),
                    2 => (D, Natural),
                    3 => (D, Sharp),
                    4 => (E, Natural),
                    5 => (F, Natural),
                    6 => (F, Sharp),
                    7 => (G, Natural),
                    8 => (G, Sharp),
                    9 => (A, Natural),
                    10 => (A, Sharp),
                    11 => (B, Natural),
                    _ => unreachable!(),
                };

                // todo: Huh?
                // let octave = (self.octave as i32 + abs / 12) as u8;
                let octave = self.octave;

                Note {
                    letter,
                    sharp_flat: Some(sharp_flat),
                    octave,
                    duration: self.duration,
                    amplitude: self.amplitude,
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum NoteDuration {
    /// Relative to a specific tick size, e.g. as set at a composition level,
    Ticks(u32),
    Traditional(NoteDurationClass),
}

impl NoteDuration {
    // pub fn get_ticks(self, tick_base: TickBase) -> u32 {
    // pub fn get_ticks(self, tick_base: NoteDurationClass) -> io::Result<u32> {
    pub fn get_ticks(self, ticks_per_sixteenth: u32) -> io::Result<u32> {
        match self {
            Self::Ticks(v) => Ok(v),
            Self::Traditional(class) => {
                let own = class as u32;
                let base = NoteDurationClass::Sixteenth as u32; // 16
                if own <= base {
                    Ok(ticks_per_sixteenth * (base / own))
                } else {
                    let divisor = own / base;
                    if ticks_per_sixteenth % divisor != 0 {
                        Err(io::Error::other(format!(
                            "Cannot represent 1/{own} note cleanly with {ticks_per_sixteenth} ticks per sixteenth"
                        )))
                    } else {
                        Ok(ticks_per_sixteenth / divisor)
                    }
                }
            }
        }
    }
}
