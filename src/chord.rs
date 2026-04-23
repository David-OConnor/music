//! Chords and chord progressions

use std::{
    fmt,
    fmt::{Display, Formatter},
};

use crate::{
    key_scale::{Key, SharpFlat},
    note::Note,
};

/// A coarse, but common way to annotate chord progressions. Does
/// not take account octave, or out-of-key intervals.
///
/// This is the "I", "IV", "V" etc of chord progressions, and is a common
/// industry abbreviated term. The numerical repr values start 1 one to match conventions.
///
/// We don't specify major/minor here (e.g. "VI" vs "vi"), as we determine that from
/// the key when creating chords from this.
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)] // todo: RM this repr if you don't use it.
pub enum ChordDegree {
    I = 1,
    II = 2,
    III = 3,
    IV = 4,
    V = 5,
    VI = 6,
    VII = 7,
}

impl ChordDegree {
    /// Returns the diatonic triad for this scale degree in the given key.
    /// Extensions and alterations are not included; use `Chord` directly for those.
    pub fn get_chord(self, key: Key, inversion: Inversion) -> Chord {
        let notes = key.get_notes();

        let (letter, sf) = notes[self as usize - 1];
        let root = Note::new(letter, Some(sf), 4);
        let quality = key.diatonic_quality(self);

        Chord::new(root, quality, None, vec![], Inversion::Root)
    }
}

/// https://en.wikipedia.org/wiki/Chord_notation
#[derive(Clone, Debug)]
pub struct Chord {
    pub root: Note,
    pub quality: ChordQuality,
    /// Highest chord extension: 7, 9, 11, or 13. `None` = plain triad.
    pub extension: Option<u8>,
    /// Chromatic alterations to individual scale degrees, e.g. `(Flat, 5)` for ♭5.
    pub alterations: Vec<(SharpFlat, u8)>,
    pub inversion: Inversion,
}

impl Display for Chord {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let root_sf = match self.root.sharp_flat {
            Some(SharpFlat::Natural) => String::new(), // Override the natural symbol.
            Some(v) => v.to_string(),
            None => String::new(),
        };

        let root_letter = match self.quality {
            ChordQuality::Major | ChordQuality::Augmented | ChordQuality::Dominant => {
                self.root.letter.to_string()
            }
            _ => self.root.letter.to_string().to_lowercase(),
        };

        match (self.quality, self.extension) {
            (ChordQuality::Major, None) => write!(f, "{}{}", root_letter, root_sf)?,
            (ChordQuality::Minor, None) => write!(f, "{}{}m", root_letter, root_sf)?,
            (ChordQuality::Augmented, None) => write!(f, "{}{}aug", root_letter, root_sf)?,
            (ChordQuality::Diminished, None) => write!(f, "{}{}dim", root_letter, root_sf)?,
            (ChordQuality::Major, Some(ext)) => write!(f, "{}{}maj{}", root_letter, root_sf, ext)?,
            (ChordQuality::Minor, Some(ext)) => write!(f, "{}{}m{}", root_letter, root_sf, ext)?,
            (ChordQuality::Augmented, Some(ext)) => {
                write!(f, "{}{}aug{}", root_letter, root_sf, ext)?
            }
            (ChordQuality::Diminished, Some(ext)) => {
                write!(f, "{}{}dim{}", root_letter, root_sf, ext)?
            }
            (ChordQuality::Dominant, None) => write!(f, "{}{}", root_letter, root_sf)?,
            (ChordQuality::Dominant, Some(ext)) => write!(f, "{}{}{}", root_letter, root_sf, ext)?,
        }

        match self.inversion {
            Inversion::Root => {}
            Inversion::First => write!(f, "\u{2076}")?, // ⁶
            Inversion::Second => write!(f, "\u{2076}\u{2084}")?, // ⁶₄
            Inversion::Third => write!(f, "\u{2076}\u{2085}")?, // ⁶₅
        }

        for (sf, deg) in &self.alterations {
            let sf_str = match sf {
                SharpFlat::Sharp => "\u{266f}",
                SharpFlat::Flat => "\u{266d}",
                SharpFlat::Natural => "\u{266e}",
            };
            write!(f, "{}{}", sf_str, deg)?;
        }
        Ok(())
    }
}

impl Chord {
    pub fn new(
        root: Note,
        quality: ChordQuality,
        extension: Option<u8>,
        alterations: Vec<(SharpFlat, u8)>,
        inversion: Inversion,
    ) -> Self {
        Self {
            root,
            quality,
            extension,
            alterations,
            inversion,
        }
    }

    pub fn with_inversion(mut self, inversion: Inversion) -> Self {
        self.inversion = inversion;
        self
    }
}

impl Chord {
    pub fn notes(&self) -> Vec<Note> {
        use ChordQuality::*;

        use crate::{key_scale::SharpFlat::*, note::NoteLetter::*};

        // Base triad intervals in semitones from root
        let mut intervals: Vec<i32> = match self.quality {
            Major | Dominant => vec![0, 4, 7],
            Minor => vec![0, 3, 7],
            Augmented => vec![0, 4, 8],
            Diminished => vec![0, 3, 6],
        };

        // Add extension tones (stacked from 7th upward)
        if let Some(ext) = self.extension {
            let seventh: i32 = match self.quality {
                Major => 11,
                Minor | Augmented | Dominant => 10,
                Diminished => 9,
            };
            if ext >= 7 {
                intervals.push(seventh);
            }
            if ext >= 9 {
                intervals.push(14);
            }
            if ext >= 11 {
                intervals.push(17);
            }
            if ext >= 13 {
                intervals.push(21);
            }
        }

        // Apply chromatic alterations: find the interval for each scale degree and shift it
        for &(sf, deg) in &self.alterations {
            let base: i32 = match deg {
                1 => 0,
                2 => 2,
                3 => match self.quality {
                    Major | Augmented | Dominant => 4,
                    Minor | Diminished => 3,
                },
                4 => 5,
                5 => match self.quality {
                    Augmented => 8,
                    Diminished => 6,
                    _ => 7,
                },
                6 => 9,
                7 => match self.quality {
                    Major => 11,
                    Minor | Augmented | Dominant => 10,
                    Diminished => 9,
                },
                9 => 14,
                11 => 17,
                13 => 21,
                _ => return vec![],
            };
            let shift: i32 = match sf {
                Sharp => 1,
                Flat => -1,
                Natural => 0,
            };
            for interval in &mut intervals {
                if *interval == base {
                    *interval += shift;
                    break;
                }
            }
        }

        let sf_offset: i32 = match self.root.sharp_flat {
            Some(Sharp) => 1,
            Some(Flat) => -1,
            Some(Natural) | None => 0,
        };
        let base_semitone: i32 = match self.root.letter {
            C => 0,
            D => 2,
            E => 4,
            F => 5,
            G => 7,
            A => 9,
            B => 11,
        } + sf_offset;

        let mut notes: Vec<Note> = intervals
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
                let octave = (self.root.octave as i32 + abs.div_euclid(12)) as u8;
                Note::new(letter, Some(sharp_flat), octave)
            })
            .collect();

        // Rotate so the inverted bass tone is first; raise the displaced notes by an octave.
        let n = match self.inversion {
            Inversion::Root => 0,
            Inversion::First => 1,
            Inversion::Second => 2,
            Inversion::Third => 3,
        };
        if n > 0 && n < notes.len() {
            let mut raised = notes[..n].to_vec();
            for note in &mut raised {
                note.octave += 1;
            }
            notes = [notes[n..].to_vec(), raised].concat();
        }

        notes
    }
}

/// The overall sound quality of the chord — combines what was previously separate
/// "type" (major/minor) and "augmentation" (augmented/diminished) fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChordQuality {
    Major,
    Minor,
    Augmented,
    Diminished,
    Dominant,
}

/// Which chord tone is in the bass (figured-bass convention).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Inversion {
    /// Root in the bass — no inversion marker.
    Root,
    /// Third in the bass — notated ⁶.
    First,
    /// Fifth in the bass — notated ⁶₄.
    Second,
    /// Seventh in the bass — notated ⁶₅ (only valid with a 7th extension).
    Third,
}

pub fn create_prog(key: Key, vals: &[(ChordDegree, Inversion)]) -> Vec<Chord> {
    vals.iter()
        .map(|(deg, inv)| deg.get_chord(key, *inv))
        .collect()
}

// todo: These are likely temp; creating common chord progressions of poplar songs
pub fn prog_1451(key: Key) -> Vec<Chord> {
    use ChordDegree::*;
    create_prog(
        key,
        &[
            (I, Inversion::Root),
            (IV, Inversion::Root),
            (V, Inversion::Root),
            (I, Inversion::Root),
        ],
    )
}

pub fn prog_1564(key: Key) -> Vec<Chord> {
    use ChordDegree::*;
    create_prog(
        key,
        &[
            (I, Inversion::Root),
            (V, Inversion::Root),
            (VI, Inversion::Root),
            (IV, Inversion::Root),
        ],
    )
}

pub fn prog_pachabel(key: Key) -> Vec<Chord> {
    use ChordDegree::*;
    create_prog(
        key,
        &[
            (I, Inversion::Root),
            (V, Inversion::Root),
            (VI, Inversion::Root),
            (III, Inversion::Root),
            (IV, Inversion::Root),
        ],
    )
}

pub fn prog_4565(key: Key) -> Vec<Chord> {
    use ChordDegree::*;
    create_prog(
        key,
        &[
            (VI, Inversion::Root),
            (V, Inversion::Root),
            (IV, Inversion::Root),
            (V, Inversion::Root),
        ],
    )
}

pub fn prog_1645(key: Key) -> Vec<Chord> {
    use ChordDegree::*;
    create_prog(
        key,
        &[
            (I, Inversion::Root),
            (VI, Inversion::Root),
            (IV, Inversion::Root),
            (V, Inversion::Root),
        ],
    )
}

pub fn prog_1465(key: Key) -> Vec<Chord> {
    use ChordDegree::*;
    create_prog(
        key,
        &[
            (I, Inversion::Root),
            (IV, Inversion::Root),
            (VI, Inversion::Root),
            (V, Inversion::Root),
        ],
    )
}

pub fn prog_1545(key: Key) -> Vec<Chord> {
    use ChordDegree::*;
    create_prog(
        key,
        &[
            (I, Inversion::Root),
            (V, Inversion::Root),
            (IV, Inversion::Root),
            (V, Inversion::Root),
        ],
    )
}

#[cfg(test)]
mod chord_tests {
    use crate::{
        chord::{Chord, ChordQuality},
        key_scale::SharpFlat,
        note::{ChordPlayed, Note, NoteDuration, NoteDurationClass, NoteLetter},
    };

    #[test]
    fn chord_notes_roll_into_the_next_octave_when_needed() {
        let chord = ChordPlayed {
            chord: Chord {
                root: Note::new(NoteLetter::B, Some(SharpFlat::Natural), 3),
                quality: ChordQuality::Major,
                extension: None,
                alterations: vec![],
                inversion: crate::chord::Inversion::Root,
            },
            duration: NoteDuration::Traditional(NoteDurationClass::Quarter),
            amplitude: 1.0,
        };

        let notes = chord.chord.notes();

        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].letter, NoteLetter::B);
        assert_eq!(notes[0].sharp_flat, Some(SharpFlat::Natural));
        assert_eq!(notes[0].octave, 3);

        assert_eq!(notes[1].letter, NoteLetter::D);
        assert_eq!(notes[1].sharp_flat, Some(SharpFlat::Sharp));
        assert_eq!(notes[1].octave, 4);

        assert_eq!(notes[2].letter, NoteLetter::F);
        assert_eq!(notes[2].sharp_flat, Some(SharpFlat::Sharp));
        assert_eq!(notes[2].octave, 4);
    }
}

#[cfg(test)]
mod chord_prog_tests {
    use crate::{
        chord::{ChordDegree, ChordQuality, Inversion},
        key_scale::{Key, MajorMinor, SharpFlat},
        note::NoteLetter,
    };

    fn c_major() -> Key {
        Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major)
    }
    fn a_minor() -> Key {
        Key::new(NoteLetter::A, SharpFlat::Natural, MajorMinor::Minor)
    }

    #[test]
    fn v_in_c_major_is_g_major() {
        let chord = ChordDegree::V.get_chord(c_major(), Inversion::Root);
        assert_eq!(chord.root.letter, NoteLetter::G);
        assert_eq!(chord.quality, ChordQuality::Major);
    }

    #[test]
    fn two_in_c_major_is_d_minor() {
        let chord = ChordDegree::II.get_chord(c_major(), Inversion::Root);
        assert_eq!(chord.root.letter, NoteLetter::D);
        assert_eq!(chord.quality, ChordQuality::Minor);
    }

    #[test]
    fn seven_in_c_major_is_b_diminished() {
        let chord = ChordDegree::VII.get_chord(c_major(), Inversion::Root);
        assert_eq!(chord.root.letter, NoteLetter::B);
        assert_eq!(chord.quality, ChordQuality::Diminished);
    }

    #[test]
    fn root_in_a_minor_is_a_minor() {
        let chord = ChordDegree::I.get_chord(a_minor(), Inversion::Root);
        assert_eq!(chord.root.letter, NoteLetter::A);
        assert_eq!(chord.quality, ChordQuality::Minor);
    }

    #[test]
    fn six_in_a_minor_is_f_major() {
        let chord = ChordDegree::VI.get_chord(a_minor(), Inversion::Root);
        assert_eq!(chord.root.letter, NoteLetter::F);
        assert_eq!(chord.quality, ChordQuality::Major);
    }
}
