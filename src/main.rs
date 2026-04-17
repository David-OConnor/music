use std::io;

use crate::{
    NoteDurationClass::Eighth,
    composition::{Composition, Measure},
    instrument::Instrument,
    overtones::Temperament,
};

mod composition;
mod decomposition;
mod generation;
mod instrument;
mod overtones;
mod player;

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

//

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
        use SharpFlat::*;

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

/// For displaying in sheet music, for example
#[derive(Clone, Copy, PartialEq)]
pub enum Clef {
    Treble,
    Bass,
    Alto,
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

/// A framework of coords, by measure. This may have more applicability to music generation
/// than as a fundamental representation of a work.
pub struct ChordProgression {
    /// Indexed by measure. These sets can be composed into a broader structure.
    /// todo: More levels?
    pub subsets: Vec<Chord>,
    /// (subset index, repetitions)
    pub sets: Vec<(usize, usize)>,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SharpFlat {
    #[default]
    Natural,
    Sharp,
    Flat,
}

/// Determined by the key.
pub struct KeySharps {
    pub a: SharpFlat,
    pub b: SharpFlat,
    pub c: SharpFlat,
    pub d: SharpFlat,
    pub e: SharpFlat,
    pub f: SharpFlat,
    pub g: SharpFlat,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Key {
    pub base_note: NoteLetter,
    pub sharp_flat: SharpFlat,
}

impl Key {
    pub fn new(base_note: NoteLetter, sharp_flat: SharpFlat) -> Key {
        Key {
            base_note,
            sharp_flat,
        }
    }

    pub fn get_sharps_flats(&self) -> KeySharps {
        use NoteLetter::*;
        use SharpFlat::*;

        // Sharps added in order: F C G D A E B
        // Flats added in order:  B E A D G C F
        let (f, c, g, d, a, e, b) = match (self.base_note, self.sharp_flat) {
            (C, Natural) => (
                Natural, Natural, Natural, Natural, Natural, Natural, Natural,
            ),
            (G, Natural) => (Sharp, Natural, Natural, Natural, Natural, Natural, Natural),
            (D, Natural) => (Sharp, Sharp, Natural, Natural, Natural, Natural, Natural),
            (A, Natural) => (Sharp, Sharp, Sharp, Natural, Natural, Natural, Natural),
            (E, Natural) => (Sharp, Sharp, Sharp, Sharp, Natural, Natural, Natural),
            (B, Natural) => (Sharp, Sharp, Sharp, Sharp, Sharp, Natural, Natural),
            (F, Sharp) => (Sharp, Sharp, Sharp, Sharp, Sharp, Sharp, Natural),
            (C, Sharp) => (Sharp, Sharp, Sharp, Sharp, Sharp, Sharp, Sharp),
            (F, Natural) => (Natural, Natural, Natural, Natural, Natural, Natural, Flat),
            (B, Flat) => (Natural, Natural, Natural, Natural, Natural, Flat, Flat),
            (E, Flat) => (Natural, Natural, Natural, Natural, Flat, Flat, Flat),
            (A, Flat) => (Natural, Natural, Natural, Flat, Flat, Flat, Flat),
            (D, Flat) => (Natural, Natural, Flat, Flat, Flat, Flat, Flat),
            (G, Flat) => (Natural, Flat, Flat, Flat, Flat, Flat, Flat),
            (C, Flat) => (Flat, Flat, Flat, Flat, Flat, Flat, Flat),
            _ => (
                Natural, Natural, Natural, Natural, Natural, Natural, Natural,
            ),
        };

        KeySharps {
            a,
            b,
            c,
            d,
            e,
            f,
            g,
        }
    }
}

// pub struct NotePlayed {
//     /// hz
//     pub pitch: f32,
//     /// seconds
//     pub duration: f32,
// }

#[derive(Clone)]
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

pub struct State {
    pub compositions: Vec<Composition>,
}

/// We are using this to develop our data structures.
/// The opening of *Alicia* from the Expedition 33 sound track.
pub fn make_test_composition() -> Composition {
    use NoteDurationClass::*;

    let instruments = vec![
        Instrument::Violin, // Treble clef
        Instrument::BassGuitar,
    ];

    let mut res = Composition::new(1, 8, instruments);

    let key = Key::new(NoteLetter::C, SharpFlat::Flat);
    let tempo = 80_000;
    let sig = TimeSignature::new(6, 8);

    let meas_0 = Measure {
        ident: 0, // Overwritten after?
        key,
        time_signature: sig,
        tempo,
    };

    // todo: How do we assign an instrument?
    // todo: Currently we could play this sequentially.
    let notes_m0 = vec![
        Note {
            letter: NoteLetter::C,
            sharp_flat: None,
            octave: 4,
            duration: NoteDuration::Traditional(Eighth),
            amplitude: 1.,
        },
        Note {
            letter: NoteLetter::G,
            sharp_flat: None,
            octave: 4,
            duration: NoteDuration::Traditional(Eighth),
            amplitude: 1.,
        },
        Note {
            letter: NoteLetter::C,
            sharp_flat: None,
            octave: 5,
            duration: NoteDuration::Traditional(Eighth),
            amplitude: 1.,
        },
        Note {
            letter: NoteLetter::D,
            sharp_flat: None,
            octave: 5,
            duration: NoteDuration::Traditional(Eighth),
            amplitude: 1.,
        },
        Note {
            letter: NoteLetter::E,
            sharp_flat: None,
            octave: 5,
            duration: NoteDuration::Traditional(Quarter),
            amplitude: 1.,
        },
    ];

    // let meas_1 = meas_0.clone();
    //
    // let measures = vec![meas_0, meas_1];
    //
    // for m in measures {
    //     res.add_measure(m);
    // }

    for note in notes_m0 {
        res.notes.push(note);
    }

    res
}

fn main() {
    let comp = make_test_composition();

    // todo: Implement a way to play this.
}
