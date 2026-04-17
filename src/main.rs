use crate::composition::{Composition, Measure};
use crate::instrument::Instrument;

mod composition;
mod instrument;
mod player;
mod decomposition;

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

pub struct Note {
    pub letter: NoteLetter,
    pub sharp_flat: SharpFlat,
    pub octave: u8,
    pub amplitude: f32,
}

impl Note {
    pub fn frequency(&self) -> f32 {
        use NoteLetter::*;
        use SharpFlat::*;

        let semitone_in_octave = match self.letter {
            C => 0,
            D => 2,
            E => 4,
            F => 5,
            G => 7,
            A => 9,
            B => 11,
        } + match self.sharp_flat {
            Sharp => 1,
            Flat => -1,
            Natural => 0,
        };

        // MIDI note: C4 = 60, A4 = 69
        let midi = (self.octave as i32 + 1) * 12 + semitone_in_octave;
        440.0 * 2f32.powf((midi - 69) as f32 / 12.0)
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
                let octave = (self.octave as i32 + abs / 12) as u8;
                Note {
                    letter,
                    sharp_flat,
                    octave,
                    amplitude: 1.0,
                }
            })
            .collect()
    }
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

#[derive(Clone, Copy)]
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

pub struct NotePlayed {
    /// hz
    pub pitch: f32,
    /// seconds
    pub duration: f32,
}

#[derive(Clone)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl TimeSignature {
    pub fn new(numerator: u8, denominator: u8) -> Self {
        Self { numerator, denominator }
    }
}

pub struct State {
    pub compositions: Vec<Composition>,
}

/// We are using this to develop our data structures.
/// The opening of *Alicia* from the Expedition 33 sound track.
pub fn make_test_composition() -> Composition {
    let instruments = vec![
        Instrument::Violin, // Treble clef
        Instrument::BassGuitar,
    ];

    let mut res = Composition::new(
        NoteDurationClass::Quarter,
        80_000,
        instruments
    );

    let key = Key::new(NoteLetter::C, SharpFlat::Flat);
    let tempo = 80_000;
    let sig = TimeSignature::new(6, 8);

    let meas_0 = Measure {
        ident: 0, // Overwritten after?
        key,
        time_signature: sig,
        tempo,
    };

    let meas_1 = meas_0.clone();

    let measures = vec![
        meas_0, meas_1
    ];

    for m in measures {
        res.add_measure(m);
    }

    res
}


fn main() {
    let comp = make_test_composition();

    // todo: Implement a way to play this.

}
