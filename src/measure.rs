//! For dividing a composition into measures.
//! This isn't required to generate a set of notes, but can help with generating, improvisation,
//! analysis etc.

use crate::note::{Chord, NoteLetter};

/// A traditional music measure.
#[derive(Clone)]
pub struct Measure {
    // todo: Do we want this? For assigning finer divisions to.
    pub ident: u32,
    pub key: Key,
    pub time_signature: TimeSignature,
    pub tempo: u32,
}

impl Measure {
    pub fn to_micro_measure(&self) -> Vec<MicroMeasure> {
        let mut res = Vec::new();

        res
    }
}

/// Describes all actions at the coarsest time granularity which can describe a given instant.
/// It describes everything which is happening at thi state. When used to play a composition,
/// for example, this is what is generated. We compose this from coarser constructs.
pub struct MicroMeasure {
    /// ms. We use the largest
    pub duration: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum SharpFlat {
    #[default]
    Natural,
    Sharp,
    Flat,
}

/// Determined by the key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeySharps {
    pub a: SharpFlat,
    pub b: SharpFlat,
    pub c: SharpFlat,
    pub d: SharpFlat,
    pub e: SharpFlat,
    pub f: SharpFlat,
    pub g: SharpFlat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MajorMinor {
    Major,
    Minor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Key {
    pub base_note: NoteLetter,
    pub sharp_flat: SharpFlat,
    pub major_minor: MajorMinor,
}

impl Key {
    pub fn new(base_note: NoteLetter, sharp_flat: SharpFlat, major_minor: MajorMinor) -> Key {
        Key {
            base_note,
            sharp_flat,
            major_minor,
        }
    }

    fn signature_count(&self) -> i8 {
        use crate::note::NoteLetter::*;
        use MajorMinor::*;

        let natural_count = match self.major_minor {
            Major => match self.base_note {
                C => 0,
                G => 1,
                D => 2,
                A => 3,
                E => 4,
                B => 5,
                F => -1,
            },
            Minor => match self.base_note {
                A => 0,
                E => 1,
                B => 2,
                D => -1,
                G => -2,
                C => -3,
                F => -4,
            },
        };

        let accidental_offset = match self.sharp_flat {
            SharpFlat::Natural => 0,
            SharpFlat::Sharp => 7,
            SharpFlat::Flat => -7,
        };

        natural_count + accidental_offset
    }

    pub fn get_sharps_flats(&self) -> KeySharps {
        use SharpFlat::*;

        let mut res = KeySharps {
            a: Natural,
            b: Natural,
            c: Natural,
            d: Natural,
            e: Natural,
            f: Natural,
            g: Natural,
        };

        // This type system only supports single sharps/flats, so theoretical keys whose
        // signatures require double accidentals cannot be represented faithfully here.
        let count = self.signature_count();
        debug_assert!(
            (-7..=7).contains(&count),
            "Key signature for {:?} {:?} {:?} needs double accidentals, which are not supported",
            self.base_note,
            self.sharp_flat,
            self.major_minor
        );

        let clamped = count.clamp(-7, 7);
        let letters = if clamped >= 0 {
            [
                ('f', Sharp),
                ('c', Sharp),
                ('g', Sharp),
                ('d', Sharp),
                ('a', Sharp),
                ('e', Sharp),
                ('b', Sharp),
            ]
        } else {
            [
                ('b', Flat),
                ('e', Flat),
                ('a', Flat),
                ('d', Flat),
                ('g', Flat),
                ('c', Flat),
                ('f', Flat),
            ]
        };

        for (letter, accidental) in letters.iter().take(clamped.unsigned_abs() as usize) {
            match letter {
                'a' => res.a = *accidental,
                'b' => res.b = *accidental,
                'c' => res.c = *accidental,
                'd' => res.d = *accidental,
                'e' => res.e = *accidental,
                'f' => res.f = *accidental,
                'g' => res.g = *accidental,
                _ => unreachable!(),
            }
        }

        res
    }
}

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

/// For displaying in sheet music, for example
#[derive(Clone, Copy, PartialEq)]
pub enum Clef {
    Treble,
    Bass,
    Alto,
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
