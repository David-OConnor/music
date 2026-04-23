//! Notes, chords etc. This is a relatively primitive set of types used in both
//! playback, and generation.

use std::{
    fmt,
    fmt::{Display, Formatter},
    io,
};

use crate::{
    chord::{Chord, ChordProgVal, ChordQuality},
    key_scale::{Key, MajorMinor, SharpFlat},
    overtones::Temperament,
};

/// For representing notes in sheet music, for example. Internally, we use integers to
/// represent durations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoteDurationClass {
    Whole,
    Half,
    HalfDotted,
    Quarter,
    QuarterDotted,
    Eighth,
    EithDotted,
    Sixteenth,
    SixteenthDotted,
    ThirtySecond,
    ThirtySecondDotted,
    SixtyFourth,
    OneTwentyEighth,
    Other(u8),
}

impl Display for NoteDurationClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Whole => write!(f, "whole"),
            Self::Half => write!(f, "half"),
            Self::HalfDotted => write!(f, "half."),
            Self::Quarter => write!(f, "quarter"),
            Self::QuarterDotted => write!(f, "quarter."),
            Self::Eighth => write!(f, "eighth"),
            Self::EithDotted => write!(f, "eighth."),
            Self::Sixteenth => write!(f, "16th"),
            Self::SixteenthDotted => write!(f, "16th."),
            Self::ThirtySecond => write!(f, "32nd"),
            Self::ThirtySecondDotted => write!(f, "32nd."),
            Self::SixtyFourth => write!(f, "64th"),
            Self::OneTwentyEighth => write!(f, "128th"),
            Self::Other(v) => write!(f, "1/{v}"),
        }
    }
}

impl NoteDurationClass {
    pub const fn val(self) -> u8 {
        match self {
            Self::Whole => 1,
            Self::Half => 2,
            Self::HalfDotted => 3,
            Self::Quarter => 4,
            Self::QuarterDotted => 6,
            Self::Eighth => 8,
            Self::EithDotted => 12,
            Self::Sixteenth => 16,
            Self::SixteenthDotted => 24,
            Self::ThirtySecond => 32,
            Self::ThirtySecondDotted => 48,
            Self::SixtyFourth => 64,
            Self::OneTwentyEighth => 128,
            Self::Other(v) => v,
        }
    }
}

/// All integer times are in ms. All frequencies are in Hz.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoteLetter {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

impl Display for NoteLetter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use NoteLetter::*;
        let v = match self {
            A => "A",
            B => "B",
            C => "C",
            D => "D",
            E => "E",
            F => "F",
            G => "G",
        };

        write!(f, "{v}")
    }
}

impl NoteLetter {
    pub fn next(self) -> NoteLetter {
        use NoteLetter::*;

        match self {
            A => B,
            B => C,
            C => D,
            D => E,
            E => F,
            F => G,
            G => A,
        }
    }

    pub fn prev(self) -> NoteLetter {
        use NoteLetter::*;

        match self {
            A => G,
            B => A,
            C => B,
            D => C,
            E => D,
            F => E,
            G => F,
        }
    }
}

/// Suitable for playing notes
#[derive(Clone, Debug)]
pub struct Note {
    pub letter: NoteLetter,
    /// If none, rever to the key.
    pub sharp_flat: Option<SharpFlat>,
    // todo: Should we break this down again, with a variant which has no octave?
    pub octave: u8,
}

impl Display for Note {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let sf = match self.sharp_flat {
            Some(SharpFlat::Sharp) => "#",
            Some(SharpFlat::Flat) => "♭",
            Some(SharpFlat::Natural) | None => "",
        };
        write!(f, "{}{}{}", self.letter, sf, self.octave)
    }
}

impl Note {
    pub fn new(letter: NoteLetter, sharp_flat: Option<SharpFlat>, octave: u8) -> Self {
        Self {
            letter,
            sharp_flat,
            octave,
        }
    }

    /// E.g C# + 2 = D#. G-flat + 3 = A. Interval is in semitones.
    pub fn add_interval(&self, interval: u8) -> Self {
        use NoteLetter::*;

        use crate::key_scale::SharpFlat::*;

        let base: i32 = match self.letter {
            C => 0,
            D => 2,
            E => 4,
            F => 5,
            G => 7,
            A => 9,
            B => 11,
        };
        let sf: i32 = match self.sharp_flat {
            Some(Sharp) => 1,
            Some(Flat) => -1,
            Some(Natural) | None => 0,
        };
        let total = base + sf + interval as i32;
        let semitone = total.rem_euclid(12);
        let octave_shift = total.div_euclid(12);
        let (letter, sharp_flat) = match semitone {
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
        Note::new(
            letter,
            Some(sharp_flat),
            (self.octave as i32 + octave_shift) as u8,
        )
    }
}

/// Suitable for playing notes
#[derive(Clone)]
pub struct NotePlayed {
    pub note: Note,
    pub duration: NoteDuration,
    pub amplitude: f32,
}

impl Display for NotePlayed {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{} - {} - {:.2}",
            self.note, self.duration, self.amplitude
        )
    }
}

impl NotePlayed {
    const fn natural_semitone(letter: NoteLetter) -> i32 {
        use NoteLetter::*;

        match letter {
            C => 0,
            D => 2,
            E => 4,
            F => 5,
            G => 7,
            A => 9,
            B => 11,
        }
    }

    fn accidental_for_key(letter: NoteLetter, key: Key) -> SharpFlat {
        use NoteLetter::*;

        let ks = key.get_sharps_flats();
        match letter {
            A => ks.a,
            B => ks.b,
            C => ks.c,
            D => ks.d,
            E => ks.e,
            F => ks.f,
            G => ks.g,
        }
    }

    fn midi_note(letter: NoteLetter, sharp_flat: SharpFlat, octave: u8) -> i32 {
        use crate::key_scale::SharpFlat::*;

        let semitone_in_octave = Self::natural_semitone(letter)
            + match sharp_flat {
                Sharp => 1,
                Flat => -1,
                Natural => 0,
            };

        // MIDI note: C4 = 60, A4 = 69
        (octave as i32 + 1) * 12 + semitone_in_octave
    }

    fn midi_frequency(midi: i32) -> f32 {
        (440.0_f64 * 2.0_f64.powf((midi - 69) as f64 / 12.0)) as f32
    }

    pub fn frequency(&self, key: Key, temperament: Temperament) -> f32 {
        use crate::key_scale::SharpFlat::*;

        let sf = match self.note.sharp_flat {
            Some(sf) => sf,
            None => Self::accidental_for_key(self.note.letter, key),
        };

        let midi = Self::midi_note(self.note.letter, sf, self.note.octave);

        match temperament {
            Temperament::Even => Self::midi_frequency(midi),
            Temperament::WellTempered(wt_key) => {
                let tonic_pc = Self::natural_semitone(wt_key.base_note)
                    + match wt_key.sharp_flat {
                        Sharp => 1,
                        Flat => -1,
                        Natural => 0,
                    };

                let interval = (midi - tonic_pc).rem_euclid(12) as usize;
                let tonic_midi = midi - interval as i32;
                let tonic_freq = Self::midi_frequency(tonic_midi);

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

#[cfg(test)]
mod tests {
    use super::{Note, NoteDuration, NoteDurationClass, NoteLetter, NotePlayed};
    use crate::{
        key_scale::{Key, MajorMinor, SharpFlat},
        overtones::Temperament,
    };

    fn note(letter: NoteLetter, sharp_flat: Option<SharpFlat>, octave: u8) -> NotePlayed {
        NotePlayed {
            note: Note::new(letter, sharp_flat, octave),
            duration: NoteDuration::Traditional(NoteDurationClass::Quarter),
            amplitude: 1.0,
        }
    }

    fn assert_close(actual: f32, expected: f32, tolerance: f32) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "expected {expected} Hz, got {actual} Hz (diff {diff})"
        );
    }

    #[test]
    fn equal_temperament_matches_reference_a4() {
        let a4 = note(NoteLetter::A, Some(SharpFlat::Natural), 4);

        assert_close(
            a4.frequency(
                Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major),
                Temperament::Even,
            ),
            440.0,
            0.0001,
        );
    }

    #[test]
    fn equal_temperament_handles_cross_octave_accidentals() {
        let c4 = note(NoteLetter::C, Some(SharpFlat::Natural), 4);
        let b_sharp_3 = note(NoteLetter::B, Some(SharpFlat::Sharp), 3);
        let b3 = note(NoteLetter::B, Some(SharpFlat::Natural), 3);
        let c_flat_4 = note(NoteLetter::C, Some(SharpFlat::Flat), 4);
        let key = Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major);

        assert_close(
            b_sharp_3.frequency(key, Temperament::Even),
            c4.frequency(key, Temperament::Even),
            0.0001,
        );
        assert_close(
            c_flat_4.frequency(key, Temperament::Even),
            b3.frequency(key, Temperament::Even),
            0.0001,
        );
    }

    #[test]
    fn well_tempered_perfect_fifth_uses_the_expected_ratio() {
        let key = Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major);
        let c4 = note(NoteLetter::C, Some(SharpFlat::Natural), 4);
        let g4 = note(NoteLetter::G, Some(SharpFlat::Natural), 4);
        let c4_freq = c4.frequency(key, Temperament::Even);

        assert_close(
            g4.frequency(key, Temperament::WellTempered(key)),
            c4_freq * (3.0 / 2.0),
            0.001,
        );
    }

    #[test]
    fn missing_accidental_uses_major_key_signature() {
        let key = Key::new(NoteLetter::F, SharpFlat::Natural, MajorMinor::Major);
        let note_from_key = note(NoteLetter::B, None, 4);
        let explicit_b_flat = note(NoteLetter::B, Some(SharpFlat::Flat), 4);

        assert_close(
            note_from_key.frequency(key, Temperament::Even),
            explicit_b_flat.frequency(key, Temperament::Even),
            0.0001,
        );
    }

    #[test]
    fn missing_accidental_uses_minor_key_signature() {
        let key = Key::new(NoteLetter::B, SharpFlat::Flat, MajorMinor::Minor);
        let note_from_key = note(NoteLetter::D, None, 4);
        let explicit_d_flat = note(NoteLetter::D, Some(SharpFlat::Flat), 4);

        assert_close(
            note_from_key.frequency(key, Temperament::Even),
            explicit_d_flat.frequency(key, Temperament::Even),
            0.0001,
        );
    }

    #[test]
    fn explicit_natural_overrides_key_signature() {
        let key = Key::new(NoteLetter::G, SharpFlat::Natural, MajorMinor::Major);
        let from_key = note(NoteLetter::F, None, 4);
        let explicit_natural = note(NoteLetter::F, Some(SharpFlat::Natural), 4);
        let explicit_sharp = note(NoteLetter::F, Some(SharpFlat::Sharp), 4);

        assert_close(
            from_key.frequency(key, Temperament::Even),
            explicit_sharp.frequency(key, Temperament::Even),
            0.0001,
        );
        assert!(
            (explicit_natural.frequency(key, Temperament::Even)
                - explicit_sharp.frequency(key, Temperament::Even))
            .abs()
                > 1.0
        );
    }
}

impl Key {
    /// Diatonic triad quality for a scale degree under major or natural minor harmony.
    pub fn diatonic_quality(&self, degree: ChordProgVal) -> ChordQuality {
        use MajorMinor::{Major as MajKey, Minor as MinKey};

        use crate::chord::{
            ChordProgVal::*,
            ChordQuality::{Diminished, Major, Minor},
        };

        match (self.major_minor, degree) {
            (MajKey, Root) | (MajKey, Four) | (MajKey, Five) => Major,
            (MajKey, Two) | (MajKey, Three) | (MajKey, Six) => Minor,
            (MajKey, Seven) => Diminished,
            (MinKey, Root) | (MinKey, Four) | (MinKey, Five) => Minor,
            (MinKey, Two) => Diminished,
            (MinKey, Three) | (MinKey, Six) | (MinKey, Seven) => Major,
        }
    }
}

/// ChordPlayed has duration and amplitude, and Chord.
pub struct ChordPlayed {
    pub chord: Chord,
    pub duration: NoteDuration,
    pub amplitude: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NoteDuration {
    /// Relative to a specific tick size, e.g. as set at a composition level,
    Ticks(u32),
    Traditional(NoteDurationClass),
}

impl Display for NoteDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let v = match self {
            Self::Ticks(v) => format!("{v} ticks"),
            Self::Traditional(class) => class.to_string(),
        };

        write!(f, "{v}")
    }
}

impl NoteDuration {
    // pub fn get_ticks(self, tick_base: TickBase) -> u32 {
    // pub fn get_ticks(self, tick_base: NoteDurationClass) -> io::Result<u32> {
    pub fn get_ticks(self, ticks_per_sixteenth: u32) -> io::Result<u32> {
        match self {
            Self::Ticks(v) => Ok(v),
            Self::Traditional(class) => {
                let own = class.val() as u32;
                let base = NoteDurationClass::Sixteenth.val() as u32; // 16
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
