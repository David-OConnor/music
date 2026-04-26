//! A/R

use std::fmt::{Display, Formatter};

use crate::note::{Note, NoteLetter};

pub struct Scale {
    pub key: Key,
    pub mode: ScaleMode,
    pub octave: u8,
}

impl Scale {
    pub fn get_notes(&self) -> [Note; 7] {
        std::array::from_fn(|i| self.get_note(i))
    }

    // todo: You could use a LUT on this if speed is a concern, and make this a const fn.
    pub fn get_note(&self, i: usize) -> Note {
        let key_notes = self.key.get_notes();
        let abs = self.mode as usize + i;
        let (letter, sf) = key_notes[abs % 7];

        Note::new(letter, Some(sf), self.octave + (abs / 7) as u8)
    }
}

/// These shift the base note. The repr is the interval shift from Ionia.
#[derive(Clone, Copy, PartialEq, Default)]
#[repr(u8)]
pub enum ScaleMode {
    /// Ionina is the base scale
    #[default]
    Ionia = 0,
    Dorian = 1,
    Phrigian = 2,
    Lydian = 3,
    Mixolydian = 4,
    Aeolian = 5,
    Locrian = 6,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum SharpFlat {
    #[default]
    Natural,
    Sharp,
    Flat,
}

impl Display for SharpFlat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let v = match self {
            SharpFlat::Natural => "♮",
            SharpFlat::Sharp => "♯",
            SharpFlat::Flat => "♭",
        };

        write!(f, "{v}")
    }
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

impl Display for MajorMinor {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let v = match self {
            Self::Major => "maj",
            Self::Minor => "min",
        };

        write!(f, "{v}")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Key {
    pub base_note: NoteLetter,
    pub sharp_flat: SharpFlat,
    pub major_minor: MajorMinor,
}

impl Default for Key {
    fn default() -> Self {
        Self {
            base_note: NoteLetter::C,
            sharp_flat: SharpFlat::default(),
            major_minor: MajorMinor::Major,
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.base_note, self.sharp_flat, self.major_minor
        )
    }
}

impl Key {
    pub fn new(base_note: NoteLetter, sharp_flat: SharpFlat, major_minor: MajorMinor) -> Key {
        Key {
            base_note,
            sharp_flat,
            major_minor,
        }
    }

    const fn signature_count(&self) -> i8 {
        use MajorMinor::*;

        use crate::note::NoteLetter::*;

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

    pub fn get_notes(&self) -> Vec<(NoteLetter, SharpFlat)> {
        use NoteLetter::*;

        let sharps = self.get_sharps_flats();
        let mut notes = Vec::with_capacity(7);
        let mut letter = self.base_note;

        for _ in 0..7 {
            let sf = match letter {
                A => sharps.a,
                B => sharps.b,
                C => sharps.c,
                D => sharps.d,
                E => sharps.e,
                F => sharps.f,
                G => sharps.g,
            };

            notes.push((letter, sf));
            letter = letter.next();
        }
        notes
    }
}
