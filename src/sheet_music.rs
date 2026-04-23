//! Generates sheet music from a composition. MusicXML to start. Perhaps PDF too.
//! Supports raw .musicxml and compressed .mxl formats.
//!
//! We may use either the `musicxml` crate, or do this directly using an XML library.
//! We will start with the library, but if it becomes easier/simpler to write and read XML directly,
//! we will do that.
//!
//! MusicXML standard: https://w3c-cg.github.io/musicxml/

use std::{io, path::Path};

use musicxml::{datatypes as mxd, elements as mx};

use crate::{
    chord::{Chord, ChordQuality, Inversion},
    composition::{Composition, NotesStartingThisTick},
    instrument::Instrument,
    key_scale::{Key, MajorMinor, SharpFlat},
    measure::{Measure, TimeSignature},
    note::{Note, NoteDuration, NoteDurationClass, NoteLetter, NotePlayed},
    overtones::Temperament,
};

#[derive(Clone, Copy, PartialEq)]
pub enum MusicXmlFormat {
    Raw,
    Compressed,
}

impl MusicXmlFormat {
    pub fn extension(self) -> String {
        String::from(match self {
            Self::Raw => "musicxml",
            Self::Compressed => "mxl",
        })
    }

    pub fn from_extension(ext: &str) -> MusicXmlFormat {
        match ext {
            "mxl" => Self::Compressed,
            _ => Self::Raw,
        }
    }
}

// --- Primitive conversions ---

fn note_letter_to_step(letter: NoteLetter) -> mxd::Step {
    match letter {
        NoteLetter::A => mxd::Step::A,
        NoteLetter::B => mxd::Step::B,
        NoteLetter::C => mxd::Step::C,
        NoteLetter::D => mxd::Step::D,
        NoteLetter::E => mxd::Step::E,
        NoteLetter::F => mxd::Step::F,
        NoteLetter::G => mxd::Step::G,
    }
}

fn step_to_note_letter(step: &mxd::Step) -> NoteLetter {
    match step {
        mxd::Step::A => NoteLetter::A,
        mxd::Step::B => NoteLetter::B,
        mxd::Step::C => NoteLetter::C,
        mxd::Step::D => NoteLetter::D,
        mxd::Step::E => NoteLetter::E,
        mxd::Step::F => NoteLetter::F,
        mxd::Step::G => NoteLetter::G,
    }
}

fn sharp_flat_to_alter(sf: Option<SharpFlat>) -> Option<mx::Alter> {
    match sf {
        Some(SharpFlat::Sharp) => Some(mx::Alter {
            attributes: (),
            content: mxd::Semitones(1),
        }),
        Some(SharpFlat::Flat) => Some(mx::Alter {
            attributes: (),
            content: mxd::Semitones(-1),
        }),
        Some(SharpFlat::Natural) | None => None,
    }
}

fn alter_to_sharp_flat(alter: Option<&mx::Alter>) -> Option<SharpFlat> {
    alter.map(|a| {
        let val = *a.content;
        if val > 0 {
            SharpFlat::Sharp
        } else if val < 0 {
            SharpFlat::Flat
        } else {
            SharpFlat::Natural
        }
    })
}

/// Counts the circle-of-fifths position for a key (positive = sharps, negative = flats).
fn key_to_fifths(key: Key) -> i8 {
    let ks = key.get_sharps_flats();
    let accidentals = [ks.f, ks.c, ks.g, ks.d, ks.a, ks.e, ks.b];
    let sharps = accidentals
        .iter()
        .filter(|&&sf| sf == SharpFlat::Sharp)
        .count() as i8;
    let flats = accidentals
        .iter()
        .filter(|&&sf| sf == SharpFlat::Flat)
        .count() as i8;
    sharps - flats
}

fn fifths_mode_to_key(fifths: i8, mode: MajorMinor) -> Key {
    use MajorMinor::*;
    use NoteLetter::*;
    use SharpFlat::*;

    let (base, sf) = match (mode, fifths) {
        (Major, 0) => (C, Natural),
        (Major, 1) => (G, Natural),
        (Major, 2) => (D, Natural),
        (Major, 3) => (A, Natural),
        (Major, 4) => (E, Natural),
        (Major, 5) => (B, Natural),
        (Major, 6) => (F, Sharp),
        (Major, 7) => (C, Sharp),
        (Major, -1) => (F, Natural),
        (Major, -2) => (B, Flat),
        (Major, -3) => (E, Flat),
        (Major, -4) => (A, Flat),
        (Major, -5) => (D, Flat),
        (Major, -6) => (G, Flat),
        (Major, -7) => (C, Flat),
        (Minor, 0) => (A, Natural),
        (Minor, 1) => (E, Natural),
        (Minor, 2) => (B, Natural),
        (Minor, 3) => (F, Sharp),
        (Minor, 4) => (C, Sharp),
        (Minor, 5) => (G, Sharp),
        (Minor, 6) => (D, Sharp),
        (Minor, 7) => (A, Sharp),
        (Minor, -1) => (D, Natural),
        (Minor, -2) => (G, Natural),
        (Minor, -3) => (C, Natural),
        (Minor, -4) => (F, Natural),
        (Minor, -5) => (B, Flat),
        (Minor, -6) => (E, Flat),
        (Minor, -7) => (A, Flat),
        _ => (C, Natural),
    };

    Key::new(base, sf, mode)
}

fn duration_class_to_type_value(class: NoteDurationClass) -> mxd::NoteTypeValue {
    match class {
        NoteDurationClass::Whole => mxd::NoteTypeValue::Whole,
        NoteDurationClass::Half | NoteDurationClass::HalfDotted => mxd::NoteTypeValue::Half,
        NoteDurationClass::Quarter | NoteDurationClass::QuarterDotted => {
            mxd::NoteTypeValue::Quarter
        }
        NoteDurationClass::Eighth | NoteDurationClass::EithDotted => mxd::NoteTypeValue::Eighth,
        NoteDurationClass::Sixteenth | NoteDurationClass::SixteenthDotted => {
            mxd::NoteTypeValue::Sixteenth
        }
        NoteDurationClass::ThirtySecond | NoteDurationClass::ThirtySecondDotted => {
            mxd::NoteTypeValue::ThirtySecond
        }
        NoteDurationClass::SixtyFourth => mxd::NoteTypeValue::SixtyFourth,
        NoteDurationClass::OneTwentyEighth => mxd::NoteTypeValue::OneHundredTwentyEighth,
        NoteDurationClass::Other(_) => mxd::NoteTypeValue::Quarter,
    }
}

fn type_value_to_duration_class(tv: &mxd::NoteTypeValue) -> NoteDurationClass {
    match tv {
        mxd::NoteTypeValue::Whole => NoteDurationClass::Whole,
        mxd::NoteTypeValue::Half => NoteDurationClass::Half,
        mxd::NoteTypeValue::Quarter => NoteDurationClass::Quarter,
        mxd::NoteTypeValue::Eighth => NoteDurationClass::Eighth,
        mxd::NoteTypeValue::Sixteenth => NoteDurationClass::Sixteenth,
        mxd::NoteTypeValue::ThirtySecond => NoteDurationClass::ThirtySecond,
        mxd::NoteTypeValue::SixtyFourth => NoteDurationClass::SixtyFourth,
        mxd::NoteTypeValue::OneHundredTwentyEighth => NoteDurationClass::OneTwentyEighth,
        _ => NoteDurationClass::Quarter,
    }
}

fn is_dotted(class: NoteDurationClass) -> bool {
    matches!(
        class,
        NoteDurationClass::HalfDotted
            | NoteDurationClass::QuarterDotted
            | NoteDurationClass::EithDotted
            | NoteDurationClass::SixteenthDotted
            | NoteDurationClass::ThirtySecondDotted
    )
}

fn instrument_clef(instr: Instrument) -> (mxd::ClefSign, i16) {
    match instr {
        Instrument::BassGuitar | Instrument::DoubleBass | Instrument::Cello => {
            (mxd::ClefSign::F, 4)
        }
        _ => (mxd::ClefSign::G, 2),
    }
}

fn instrument_has_grand_staff(instr: Instrument) -> bool {
    matches!(instr, Instrument::Piano)
}

fn instrument_name(instr: Instrument) -> String {
    match instr {
        Instrument::Piano => "Piano",
        Instrument::Guitar => "Guitar",
        Instrument::BassGuitar => "Bass Guitar",
        Instrument::Drums => "Drums",
        Instrument::Violin => "Violin",
        Instrument::Viola => "Viola",
        Instrument::Cello => "Cello",
        Instrument::DoubleBass => "Double Bass",
        Instrument::Trumpet => "Trumpet",
        Instrument::Saxophone => "Saxophone",
        Instrument::Flute => "Flute",
        Instrument::Oboe => "Oboe",
        Instrument::Clarinet => "Clarinet",
        Instrument::Banjo => "Banjo",
    }
    .to_string()
}

// --- Harmony / chord symbol conversions ---

fn sharp_flat_to_root_alter(sf: Option<SharpFlat>) -> Option<mx::RootAlter> {
    match sf {
        Some(SharpFlat::Sharp) => Some(mx::RootAlter {
            attributes: mx::RootAlterAttributes::default(),
            content: mxd::Semitones(1),
        }),
        Some(SharpFlat::Flat) => Some(mx::RootAlter {
            attributes: mx::RootAlterAttributes::default(),
            content: mxd::Semitones(-1),
        }),
        Some(SharpFlat::Natural) | None => None,
    }
}

fn chord_to_kind_value(chord: &Chord) -> mxd::KindValue {
    match (chord.quality, chord.extension) {
        (ChordQuality::Major, None) => mxd::KindValue::Major,
        (ChordQuality::Major, Some(7)) => mxd::KindValue::MajorSeventh,
        (ChordQuality::Major, Some(9)) => mxd::KindValue::MajorNinth,
        (ChordQuality::Major, Some(11)) => mxd::KindValue::Major11th,
        (ChordQuality::Major, Some(13)) => mxd::KindValue::Major13th,
        (ChordQuality::Minor, None) => mxd::KindValue::Minor,
        (ChordQuality::Minor, Some(7)) => mxd::KindValue::MinorSeventh,
        (ChordQuality::Minor, Some(9)) => mxd::KindValue::MinorNinth,
        (ChordQuality::Minor, Some(11)) => mxd::KindValue::Minor11th,
        (ChordQuality::Minor, Some(13)) => mxd::KindValue::Minor13th,
        (ChordQuality::Augmented, None) => mxd::KindValue::Augmented,
        (ChordQuality::Augmented, Some(7)) => mxd::KindValue::AugmentedSeventh,
        (ChordQuality::Diminished, None) => mxd::KindValue::Diminished,
        (ChordQuality::Diminished, Some(7)) => mxd::KindValue::DiminishedSeventh,
        _ => mxd::KindValue::Major,
    }
}

fn kind_value_to_quality_extension(kv: &mxd::KindValue) -> (ChordQuality, Option<u8>) {
    use mxd::KindValue::{
        Augmented, AugmentedSeventh, Diminished, DiminishedSeventh, Dominant, Dominant11th,
        Dominant13th, DominantNinth, HalfDiminished, Major, Major11th, Major13th, MajorMinor,
        MajorNinth, MajorSeventh, Minor, Minor11th, Minor13th, MinorNinth, MinorSeventh,
        MinorSixth, Power,
    };
    match kv {
        Major | Power => (ChordQuality::Major, Option::None),
        MajorSeventh => (ChordQuality::Major, Some(7)),
        MajorNinth => (ChordQuality::Major, Some(9)),
        Major11th => (ChordQuality::Major, Some(11)),
        Major13th => (ChordQuality::Major, Some(13)),
        Minor | MinorSixth => (ChordQuality::Minor, Option::None),
        MinorSeventh | MajorMinor => (ChordQuality::Minor, Some(7)),
        MinorNinth => (ChordQuality::Minor, Some(9)),
        Minor11th => (ChordQuality::Minor, Some(11)),
        Minor13th => (ChordQuality::Minor, Some(13)),
        Dominant => (ChordQuality::Major, Some(7)),
        DominantNinth => (ChordQuality::Major, Some(9)),
        Dominant11th => (ChordQuality::Major, Some(11)),
        Dominant13th => (ChordQuality::Major, Some(13)),
        Augmented => (ChordQuality::Augmented, Option::None),
        AugmentedSeventh => (ChordQuality::Augmented, Some(7)),
        Diminished => (ChordQuality::Diminished, Option::None),
        DiminishedSeventh | HalfDiminished => (ChordQuality::Diminished, Some(7)),
        _ => (ChordQuality::Major, Option::None),
    }
}

fn chord_to_harmony(chord: &Chord) -> mx::Harmony {
    let degrees: Vec<mx::Degree> = chord
        .alterations
        .iter()
        .map(|(sf, deg)| mx::Degree {
            attributes: mx::DegreeAttributes::default(),
            content: mx::DegreeContents {
                degree_value: mx::DegreeValue {
                    attributes: mx::DegreeValueAttributes::default(),
                    content: mxd::PositiveInteger(*deg as u32),
                },
                degree_alter: mx::DegreeAlter {
                    attributes: mx::DegreeAlterAttributes::default(),
                    content: mxd::Semitones(match sf {
                        SharpFlat::Sharp => 1,
                        SharpFlat::Flat => -1,
                        SharpFlat::Natural => 0,
                    }),
                },
                degree_type: mx::DegreeType {
                    attributes: mx::DegreeTypeAttributes::default(),
                    content: mxd::DegreeTypeValue::Alter,
                },
            },
        })
        .collect();

    mx::Harmony {
        attributes: mx::HarmonyAttributes::default(),
        content: mx::HarmonyContents {
            harmony: vec![mx::HarmonySubcontents {
                root: Some(mx::Root {
                    attributes: (),
                    content: mx::RootContents {
                        root_step: mx::RootStep {
                            attributes: mx::RootStepAttributes::default(),
                            content: note_letter_to_step(chord.root.letter),
                        },
                        root_alter: sharp_flat_to_root_alter(chord.root.sharp_flat),
                    },
                }),
                numeral: None,
                function: None,
                kind: mx::Kind {
                    attributes: mx::KindAttributes::default(),
                    content: chord_to_kind_value(chord),
                },
                inversion: None,
                bass: None,
                degree: degrees,
            }],
            frame: None,
            offset: None,
            footnote: None,
            level: None,
            staff: None,
        },
    }
}

fn harmony_to_chord(h: &mx::Harmony) -> Option<Chord> {
    let sub = h.content.harmony.first()?;
    let root_el = sub.root.as_ref()?;
    let letter = step_to_note_letter(&root_el.content.root_step.content);
    let sf = root_el.content.root_alter.as_ref().map(|ra| {
        let val = *ra.content;
        if val > 0 {
            SharpFlat::Sharp
        } else if val < 0 {
            SharpFlat::Flat
        } else {
            SharpFlat::Natural
        }
    });
    let (quality, extension) = kind_value_to_quality_extension(&sub.kind.content);
    let alterations: Vec<(SharpFlat, u8)> = sub
        .degree
        .iter()
        .filter_map(|d| {
            let deg = *d.content.degree_value.content as u8;
            let alter = *d.content.degree_alter.content;
            let sf = if alter > 0 {
                SharpFlat::Sharp
            } else if alter < 0 {
                SharpFlat::Flat
            } else {
                SharpFlat::Natural
            };
            Some((sf, deg))
        })
        .collect();
    Some(Chord::new(
        Note::new(letter, sf, 4),
        quality,
        extension,
        alterations,
        Inversion::Root,
    ))
}

// --- Tick / division arithmetic ---

/// MusicXML divisions per quarter note = 4 × ticks_per_sixteenth.
/// This makes 1 tick = 1 MusicXML division, simplifying all conversions.
fn divs_per_quarter(ticks_per_sixteenth: u32) -> u32 {
    ticks_per_sixteenth * 4
}

/// Convert `NoteDuration` to MusicXML division count (1 tick = 1 division).
fn note_dur_to_divs(dur: NoteDuration, ticks_per_sixteenth: u32) -> u32 {
    dur.get_ticks(ticks_per_sixteenth)
        .unwrap_or(ticks_per_sixteenth * 4)
}

/// Best-fit (NoteTypeValue, is_dotted) for a given division count.
fn divs_to_note_type(divs: u32, dpq: u32) -> (mxd::NoteTypeValue, bool) {
    if dpq == 0 {
        return (mxd::NoteTypeValue::Quarter, false);
    }
    // Check exact matches from largest to smallest, including dotted values.
    if divs == dpq * 4 {
        (mxd::NoteTypeValue::Whole, false)
    } else if divs == dpq * 3 {
        (mxd::NoteTypeValue::Half, true)
    } else if divs == dpq * 2 {
        (mxd::NoteTypeValue::Half, false)
    } else if dpq >= 2 && divs == dpq * 3 / 2 {
        (mxd::NoteTypeValue::Quarter, true)
    } else if divs == dpq {
        (mxd::NoteTypeValue::Quarter, false)
    } else if dpq >= 4 && divs == dpq * 3 / 4 {
        (mxd::NoteTypeValue::Eighth, true)
    } else if dpq >= 2 && divs == dpq / 2 {
        (mxd::NoteTypeValue::Eighth, false)
    } else if dpq >= 4 && divs == dpq / 4 {
        (mxd::NoteTypeValue::Sixteenth, false)
    } else if dpq >= 8 && divs == dpq / 8 {
        (mxd::NoteTypeValue::ThirtySecond, false)
    } else {
        (mxd::NoteTypeValue::Quarter, false)
    }
}

// --- Note builders ---

fn make_rest(divs: u32, dpq: u32, staff_num: Option<u8>, voice: &str) -> mx::Note {
    let (type_val, dotted) = divs_to_note_type(divs, dpq);
    let dot = if dotted {
        vec![mx::Dot {
            attributes: mx::DotAttributes::default(),
            content: (),
        }]
    } else {
        vec![]
    };
    mx::Note {
        attributes: mx::NoteAttributes::default(),
        content: mx::NoteContents {
            info: mx::NoteType::Normal(mx::NormalInfo {
                chord: None,
                audible: mx::AudibleType::Rest(mx::Rest {
                    attributes: mx::RestAttributes::default(),
                    content: mx::RestContents {
                        display_step: None,
                        display_octave: None,
                    },
                }),
                duration: mx::Duration {
                    attributes: (),
                    content: mxd::PositiveDivisions(divs),
                },
                tie: vec![],
            }),
            instrument: vec![],
            footnote: None,
            level: None,
            voice: Some(mx::Voice {
                attributes: (),
                content: voice.to_string(),
            }),
            r#type: Some(mx::Type {
                attributes: mx::TypeAttributes::default(),
                content: type_val,
            }),
            dot,
            accidental: None,
            time_modification: None,
            stem: None,
            notehead: None,
            notehead_text: None,
            staff: staff_num.map(|s| mx::Staff {
                attributes: (),
                content: mxd::PositiveInteger(s as u32),
            }),
            beam: vec![],
            notations: vec![],
            lyric: vec![],
            play: None,
            listen: None,
        },
    }
}

/// Greedy fill: decompose `gap_divs` into a sequence of rest notes from largest to smallest.
fn fill_rests(gap_divs: u32, dpq: u32, staff_num: Option<u8>, voice: &str) -> Vec<mx::Note> {
    if dpq == 0 || gap_divs == 0 {
        return vec![];
    }
    let mut remaining = gap_divs;
    let mut notes = vec![];
    // Standard durations from whole down to 32nd, including dotted variants
    let dw = dpq * 4;
    let dhd = dpq * 3;
    let dh = dpq * 2;
    let dqd = if dpq >= 2 { dpq * 3 / 2 } else { 0 };
    let dq = dpq;
    let d8d = if dpq >= 4 { dpq * 3 / 4 } else { 0 };
    let d8 = if dpq >= 2 { dpq / 2 } else { 0 };
    let d16 = if dpq >= 4 { dpq / 4 } else { 0 };
    let d32 = if dpq >= 8 { dpq / 8 } else { 0 };
    for &d in &[dw, dhd, dh, dqd, dq, d8d, d8, d16, d32] {
        if d == 0 {
            continue;
        }
        while remaining >= d {
            notes.push(make_rest(d, dpq, staff_num, voice));
            remaining -= d;
        }
        if remaining == 0 {
            break;
        }
    }
    notes
}

fn make_pitch_note(
    note: &NotePlayed,
    is_chord: bool,
    ticks_per_sixteenth: u32,
    staff_num: Option<u8>,
    voice: &str,
) -> mx::Note {
    let dpq = divs_per_quarter(ticks_per_sixteenth);
    let divs = note_dur_to_divs(note.duration, ticks_per_sixteenth);

    let (type_val, dotted) = match note.duration {
        NoteDuration::Traditional(class) => (duration_class_to_type_value(class), is_dotted(class)),
        NoteDuration::Ticks(_) => divs_to_note_type(divs, dpq),
    };

    let dot = if dotted {
        vec![mx::Dot {
            attributes: mx::DotAttributes::default(),
            content: (),
        }]
    } else {
        vec![]
    };

    mx::Note {
        attributes: mx::NoteAttributes::default(),
        content: mx::NoteContents {
            info: mx::NoteType::Normal(mx::NormalInfo {
                chord: if is_chord {
                    Some(mx::Chord {
                        attributes: (),
                        content: (),
                    })
                } else {
                    None
                },
                audible: mx::AudibleType::Pitch(mx::Pitch {
                    attributes: (),
                    content: mx::PitchContents {
                        step: mx::Step {
                            attributes: (),
                            content: note_letter_to_step(note.note.letter),
                        },
                        alter: sharp_flat_to_alter(note.note.sharp_flat),
                        octave: mx::Octave {
                            attributes: (),
                            content: mxd::Octave(note.note.octave.min(9)),
                        },
                    },
                }),
                duration: mx::Duration {
                    attributes: (),
                    content: mxd::PositiveDivisions(divs),
                },
                tie: vec![],
            }),
            instrument: vec![],
            footnote: None,
            level: None,
            voice: Some(mx::Voice {
                attributes: (),
                content: voice.to_string(),
            }),
            r#type: Some(mx::Type {
                attributes: mx::TypeAttributes::default(),
                content: type_val,
            }),
            dot,
            accidental: None,
            time_modification: None,
            stem: None,
            notehead: None,
            notehead_text: None,
            staff: staff_num.map(|s| mx::Staff {
                attributes: (),
                content: mxd::PositiveInteger(s as u32),
            }),
            beam: vec![],
            notations: vec![],
            lyric: vec![],
            play: None,
            listen: None,
        },
    }
}

// --- Measure / score builders ---

fn make_measure_attrs(key: Key, ts: &TimeSignature, instr: Instrument, dpq: u32) -> mx::Attributes {
    let grand = instrument_has_grand_staff(instr);
    let mode_val = match key.major_minor {
        MajorMinor::Major => mxd::Mode::Major,
        MajorMinor::Minor => mxd::Mode::Minor,
    };

    let clefs = if grand {
        vec![
            mx::Clef {
                attributes: mx::ClefAttributes {
                    number: Some(mxd::StaffNumber(1)),
                    ..Default::default()
                },
                content: mx::ClefContents {
                    sign: mx::Sign {
                        attributes: (),
                        content: mxd::ClefSign::G,
                    },
                    line: Some(mx::Line {
                        attributes: (),
                        content: mxd::StaffLinePosition(2),
                    }),
                    clef_octave_change: None,
                },
            },
            mx::Clef {
                attributes: mx::ClefAttributes {
                    number: Some(mxd::StaffNumber(2)),
                    ..Default::default()
                },
                content: mx::ClefContents {
                    sign: mx::Sign {
                        attributes: (),
                        content: mxd::ClefSign::F,
                    },
                    line: Some(mx::Line {
                        attributes: (),
                        content: mxd::StaffLinePosition(4),
                    }),
                    clef_octave_change: None,
                },
            },
        ]
    } else {
        let (clef_sign, clef_line) = instrument_clef(instr);
        vec![mx::Clef {
            attributes: mx::ClefAttributes::default(),
            content: mx::ClefContents {
                sign: mx::Sign {
                    attributes: (),
                    content: clef_sign,
                },
                line: Some(mx::Line {
                    attributes: (),
                    content: mxd::StaffLinePosition(clef_line),
                }),
                clef_octave_change: None,
            },
        }]
    };

    mx::Attributes {
        attributes: (),
        content: mx::AttributesContents {
            footnote: None,
            level: None,
            divisions: Some(mx::Divisions {
                attributes: (),
                content: mxd::PositiveDivisions(dpq),
            }),
            key: vec![mx::Key {
                attributes: mx::KeyAttributes::default(),
                content: mx::KeyContents::Explicit(mx::ExplicitKeyContents {
                    cancel: None,
                    fifths: mx::Fifths {
                        attributes: (),
                        content: mxd::Fifths(key_to_fifths(key)),
                    },
                    mode: Some(mx::Mode {
                        attributes: (),
                        content: mode_val,
                    }),
                    key_octave: vec![],
                }),
            }],
            time: vec![mx::Time {
                attributes: mx::TimeAttributes::default(),
                content: mx::TimeContents {
                    beats: vec![mx::TimeBeatContents {
                        beats: mx::Beats {
                            attributes: (),
                            content: ts.numerator.to_string(),
                        },
                        beat_type: mx::BeatType {
                            attributes: (),
                            content: ts.denominator.to_string(),
                        },
                    }],
                    interchangeable: None,
                    senza_misura: None,
                },
            }],
            staves: if grand {
                Some(mx::Staves {
                    attributes: (),
                    content: mxd::NonNegativeInteger(2),
                })
            } else {
                None
            },
            part_symbol: None,
            instruments: None,
            clef: clefs,
            staff_details: vec![],
            transpose: vec![],
            for_part: vec![],
            directive: vec![],
            measure_style: vec![],
        },
    }
}

/// Total ticks (= MusicXML divisions) that a measure occupies.
fn ticks_per_measure(ts: &TimeSignature, ticks_per_sixteenth: u32) -> usize {
    (ticks_per_sixteenth * 16 * ts.numerator as u32 / ts.denominator as u32) as usize
}

/// Build notes for a single staff, filtering `notes_by_tick` by `staff_filter`.
/// `staff_filter = None` includes all notes (single-staff instruments).
/// `staff_filter = Some(n)` includes only notes with `staff == Some(n)` (or `None` treated as 1).
fn build_staff_notes(
    tick_start: usize,
    tick_count: usize,
    notes_by_tick: &[NotesStartingThisTick],
    ticks_per_sixteenth: u32,
    staff_filter: Option<u8>,
    voice: &str,
) -> Vec<mx::MeasureElement> {
    let dpq = divs_per_quarter(ticks_per_sixteenth);
    let tick_end = (tick_start + tick_count).min(notes_by_tick.len());
    let mut elements = vec![];
    let mut filled: u32 = 0;

    for tick_idx in tick_start..tick_end {
        let group = &notes_by_tick[tick_idx];
        let filtered: Vec<&NotePlayed> = match staff_filter {
            None => group.notes.iter().collect(),
            Some(sf) => group
                .notes
                .iter()
                .filter(|n| n.staff.unwrap_or(1) == sf)
                .collect(),
        };

        if filtered.is_empty() {
            continue;
        }

        let tick_divs = (tick_idx - tick_start) as u32;
        if tick_divs > filled {
            for rest in fill_rests(tick_divs - filled, dpq, staff_filter, voice) {
                elements.push(mx::MeasureElement::Note(rest));
            }
            filled = tick_divs;
        }

        let first_dur = note_dur_to_divs(filtered[0].duration, ticks_per_sixteenth);
        for (i, note) in filtered.iter().enumerate() {
            elements.push(mx::MeasureElement::Note(make_pitch_note(
                note,
                i > 0,
                ticks_per_sixteenth,
                staff_filter,
                voice,
            )));
        }
        filled += first_dur;
    }

    let measure_total = tick_count as u32;
    if filled < measure_total {
        for rest in fill_rests(measure_total - filled, dpq, staff_filter, voice) {
            elements.push(mx::MeasureElement::Note(rest));
        }
    }

    elements
}

/// Build the note elements for a measure slice of `notes_by_tick`.
fn build_measure_notes(
    tick_start: usize,
    tick_count: usize,
    notes_by_tick: &[NotesStartingThisTick],
    ticks_per_sixteenth: u32,
    grand_staff: bool,
) -> Vec<mx::MeasureElement> {
    if grand_staff {
        let mut elements = build_staff_notes(
            tick_start,
            tick_count,
            notes_by_tick,
            ticks_per_sixteenth,
            Some(1),
            "1",
        );
        elements.push(mx::MeasureElement::Backup(mx::Backup {
            attributes: (),
            content: mx::BackupContents {
                duration: mx::Duration {
                    attributes: (),
                    content: mxd::PositiveDivisions(tick_count as u32),
                },
                footnote: None,
                level: None,
            },
        }));
        elements.extend(build_staff_notes(
            tick_start,
            tick_count,
            notes_by_tick,
            ticks_per_sixteenth,
            Some(2),
            "2",
        ));
        elements
    } else {
        build_staff_notes(
            tick_start,
            tick_count,
            notes_by_tick,
            ticks_per_sixteenth,
            None,
            "1",
        )
    }
}

fn composition_to_score(comp: &Composition) -> mx::ScorePartwise {
    let part_id = "P1";
    let instr = comp
        .instruments
        .first()
        .copied()
        .unwrap_or(Instrument::Piano);
    let dpq = divs_per_quarter(comp.ticks_per_sixteenth_note);

    let part_list = mx::PartList {
        attributes: (),
        content: mx::PartListContents {
            content: vec![mx::PartListElement::ScorePart(mx::ScorePart {
                attributes: mx::ScorePartAttributes {
                    id: mxd::Id(part_id.to_string()),
                },
                content: mx::ScorePartContents {
                    identification: None,
                    part_link: vec![],
                    part_name: mx::PartName {
                        attributes: mx::PartNameAttributes::default(),
                        content: instrument_name(instr),
                    },
                    part_name_display: None,
                    part_abbreviation: None,
                    part_abbreviation_display: None,
                    group: vec![],
                    score_instrument: vec![],
                    player: vec![],
                    midi_device: vec![],
                    midi_instrument: vec![],
                },
            })],
        },
    };

    let mut part_content: Vec<mx::PartElement> = vec![];

    let grand = instrument_has_grand_staff(instr);

    if comp.measures.is_empty() {
        let ts = TimeSignature::new(4, 4);
        let attrs = make_measure_attrs(comp.key, &ts, instr, dpq);
        let mut measure_content = vec![mx::MeasureElement::Attributes(attrs)];
        measure_content.extend(build_measure_notes(
            0,
            comp.notes_by_tick.len(),
            &comp.notes_by_tick,
            comp.ticks_per_sixteenth_note,
            grand,
        ));
        part_content.push(mx::PartElement::Measure(mx::Measure {
            attributes: mx::MeasureAttributes {
                number: mxd::Token("1".to_string()),
                id: None,
                implicit: None,
                non_controlling: None,
                text: None,
                width: None,
            },
            content: measure_content,
        }));
    } else {
        let mut tick_start = 0;
        for (i, measure) in comp.measures.iter().enumerate() {
            let tick_count =
                ticks_per_measure(&measure.time_signature, comp.ticks_per_sixteenth_note);
            let mut measure_content = vec![];
            if i == 0 {
                let attrs = make_measure_attrs(comp.key, &measure.time_signature, instr, dpq);
                measure_content.push(mx::MeasureElement::Attributes(attrs));
            }
            if let Some(ref chord) = measure.chord {
                measure_content.push(mx::MeasureElement::Harmony(chord_to_harmony(chord)));
            }
            measure_content.extend(build_measure_notes(
                tick_start,
                tick_count,
                &comp.notes_by_tick,
                comp.ticks_per_sixteenth_note,
                grand,
            ));
            part_content.push(mx::PartElement::Measure(mx::Measure {
                attributes: mx::MeasureAttributes {
                    number: mxd::Token((i + 1).to_string()),
                    id: None,
                    implicit: None,
                    non_controlling: None,
                    text: None,
                    width: None,
                },
                content: measure_content,
            }));
            tick_start += tick_count;
        }
    }

    mx::ScorePartwise {
        attributes: mx::ScorePartwiseAttributes {
            version: Some(mxd::Token("4.0".to_string())),
        },
        content: mx::ScorePartwiseContents {
            work: None,
            movement_number: None,
            movement_title: None,
            identification: None,
            defaults: None,
            credit: vec![],
            part_list,
            part: vec![mx::Part {
                attributes: mx::PartAttributes {
                    id: mxd::IdRef(part_id.to_string()),
                },
                content: part_content,
            }],
        },
    }
}

fn score_to_composition(score: &mx::ScorePartwise) -> Composition {
    let mut key = Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major);
    let mut ts = TimeSignature::new(4, 4);
    let mut dpq = 4u32;

    // Pull key, time sig, and divisions from the first measure's Attributes element
    'find_attrs: for part in &score.content.part {
        for elem in &part.content {
            if let mx::PartElement::Measure(m) = elem {
                for mel in &m.content {
                    if let mx::MeasureElement::Attributes(attrs) = mel {
                        if let Some(d) = &attrs.content.divisions {
                            dpq = *d.content;
                        }
                        if let Some(key_el) = attrs.content.key.first() {
                            if let mx::KeyContents::Explicit(exp) = &key_el.content {
                                let fifths = *exp.fifths.content;
                                let mode = match &exp.mode {
                                    Some(m) if m.content == mxd::Mode::Minor => MajorMinor::Minor,
                                    _ => MajorMinor::Major,
                                };
                                key = fifths_mode_to_key(fifths, mode);
                            }
                        }
                        if let Some(time_el) = attrs.content.time.first() {
                            if let Some(tb) = time_el.content.beats.first() {
                                if let (Ok(num), Ok(den)) = (
                                    tb.beats.content.parse::<u8>(),
                                    tb.beat_type.content.parse::<u8>(),
                                ) {
                                    ts = TimeSignature::new(num, den);
                                }
                            }
                        }
                    }
                }
                break 'find_attrs;
            }
        }
    }

    let ticks_per_sixteenth = (dpq / 4).max(1);
    // At 120 BPM: 500 ms per quarter note / dpq divisions per quarter = ms per division (= tick)
    let ms_per_tick = (500 / dpq).max(1);

    let mut comp = Composition::new(
        ticks_per_sixteenth,
        ms_per_tick,
        key,
        Temperament::Even,
        vec![],
    );

    // Extract notes and measures from all parts
    for (part_idx, part) in score.content.part.iter().enumerate() {
        let mut current_tick: usize = 0;
        let mut current_key = key;
        let mut current_ts = ts;

        for elem in &part.content {
            if let mx::PartElement::Measure(measure) = elem {
                let mut measure_chord: Option<Chord> = None;

                for mel in &measure.content {
                    match mel {
                        mx::MeasureElement::Attributes(attrs) => {
                            if let Some(key_el) = attrs.content.key.first() {
                                if let mx::KeyContents::Explicit(exp) = &key_el.content {
                                    let fifths = *exp.fifths.content;
                                    let mode = match &exp.mode {
                                        Some(m) if m.content == mxd::Mode::Minor => {
                                            MajorMinor::Minor
                                        }
                                        _ => MajorMinor::Major,
                                    };
                                    current_key = fifths_mode_to_key(fifths, mode);
                                }
                            }
                            if let Some(time_el) = attrs.content.time.first() {
                                if let Some(tb) = time_el.content.beats.first() {
                                    if let (Ok(num), Ok(den)) = (
                                        tb.beats.content.parse::<u8>(),
                                        tb.beat_type.content.parse::<u8>(),
                                    ) {
                                        current_ts = TimeSignature::new(num, den);
                                    }
                                }
                            }
                        }
                        mx::MeasureElement::Harmony(h) => {
                            measure_chord = harmony_to_chord(h);
                        }
                        mx::MeasureElement::Note(note) => {
                            if let mx::NoteType::Normal(info) = &note.content.info {
                                let dur_divs = *info.duration.content as usize;
                                let is_chord = info.chord.is_some();

                                match &info.audible {
                                    mx::AudibleType::Pitch(pitch) => {
                                        let letter =
                                            step_to_note_letter(&pitch.content.step.content);
                                        let octave = *pitch.content.octave.content;
                                        let sf = alter_to_sharp_flat(pitch.content.alter.as_ref());

                                        let duration = if let Some(type_el) = &note.content.r#type {
                                            let class =
                                                type_value_to_duration_class(&type_el.content);
                                            let has_dot = !note.content.dot.is_empty();
                                            if has_dot {
                                                match class {
                                                    NoteDurationClass::Half => {
                                                        NoteDuration::Traditional(
                                                            NoteDurationClass::HalfDotted,
                                                        )
                                                    }
                                                    NoteDurationClass::Quarter => {
                                                        NoteDuration::Traditional(
                                                            NoteDurationClass::QuarterDotted,
                                                        )
                                                    }
                                                    NoteDurationClass::Eighth => {
                                                        NoteDuration::Traditional(
                                                            NoteDurationClass::EithDotted,
                                                        )
                                                    }
                                                    NoteDurationClass::Sixteenth => {
                                                        NoteDuration::Traditional(
                                                            NoteDurationClass::SixteenthDotted,
                                                        )
                                                    }
                                                    NoteDurationClass::ThirtySecond => {
                                                        NoteDuration::Traditional(
                                                            NoteDurationClass::ThirtySecondDotted,
                                                        )
                                                    }
                                                    _ => NoteDuration::Ticks(dur_divs as u32),
                                                }
                                            } else {
                                                NoteDuration::Traditional(class)
                                            }
                                        } else {
                                            NoteDuration::Ticks(dur_divs as u32)
                                        };

                                        let tick = current_tick;
                                        while comp.notes_by_tick.len() <= tick {
                                            comp.notes_by_tick.push(NotesStartingThisTick::empty());
                                        }
                                        comp.notes_by_tick[tick].notes.push(NotePlayed {
                                            note: Note::new(letter, sf, octave),
                                            duration,
                                            amplitude: 0.8,
                                            staff: None,
                                        });

                                        if !is_chord {
                                            current_tick += dur_divs;
                                        }
                                    }
                                    mx::AudibleType::Rest(_) => {
                                        if !is_chord {
                                            current_tick += dur_divs;
                                        }
                                    }
                                    mx::AudibleType::Unpitched(_) => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if part_idx == 0 {
                    comp.measures
                        .push(Measure::new(current_key, current_ts, measure_chord, 120));
                }
            }
        }
    }

    comp
}

// --- Public API ---

pub fn write_sheet_music(
    comp: &Composition,
    format: MusicXmlFormat,
    path: &Path,
) -> io::Result<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF-8 path"))?;
    let score = composition_to_score(comp);
    let compressed = format == MusicXmlFormat::Compressed;
    musicxml::write_partwise_score(path_str, &score, compressed, false)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub fn read_sheet_music(path: &Path) -> io::Result<Composition> {
    let path_str = path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF-8 path"))?;
    let score = musicxml::read_score_partwise(path_str)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(score_to_composition(&score))
}
