//! Create Music XML from compositions, and vice versa.
//! Supports raw .musicxml and compressed .mxl formats.
//!
//! MusicXML standard: https://w3c-cg.github.io/musicxml/

use std::{fs, io, path::Path};

use musicxml::{datatypes as mxd, elements as mx};

use crate::{
    chord::{Chord, ChordQuality, Inversion},
    composition::{CompMetadata, Composition},
    instrument::Instrument,
    key_scale::{Key, MajorMinor, SharpFlat},
    measure::{Measure, Staff, TimeSignature},
    note::{Note, NoteEngraving, NoteLetter, NotePlayed},
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

const MUSICXML_3_PARTWISE_DOCTYPE: &str = "<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 3.0 Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">";
const MUSICXML_4_PARTWISE_DOCTYPE: &str = "<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">";

fn normalize_raw_musicxml_doctype(xml: String) -> String {
    xml.replacen(MUSICXML_3_PARTWISE_DOCTYPE, MUSICXML_4_PARTWISE_DOCTYPE, 1)
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

fn duration_class_to_type_value(class: NoteEngraving) -> mxd::NoteTypeValue {
    match class {
        NoteEngraving::Whole => mxd::NoteTypeValue::Whole,
        NoteEngraving::Half | NoteEngraving::HalfDotted => mxd::NoteTypeValue::Half,
        NoteEngraving::Quarter | NoteEngraving::QuarterDotted => mxd::NoteTypeValue::Quarter,
        NoteEngraving::Eighth | NoteEngraving::EithDotted => mxd::NoteTypeValue::Eighth,
        NoteEngraving::Sixteenth | NoteEngraving::SixteenthDotted => mxd::NoteTypeValue::Sixteenth,
        NoteEngraving::ThirtySecond | NoteEngraving::ThirtySecondDotted => {
            mxd::NoteTypeValue::ThirtySecond
        }
        NoteEngraving::SixtyFourth => mxd::NoteTypeValue::SixtyFourth,
        NoteEngraving::OneTwentyEighth => mxd::NoteTypeValue::OneHundredTwentyEighth,
        NoteEngraving::Other(_) => mxd::NoteTypeValue::Quarter,
    }
}

fn type_value_to_duration_class(tv: &mxd::NoteTypeValue) -> NoteEngraving {
    match tv {
        mxd::NoteTypeValue::Whole => NoteEngraving::Whole,
        mxd::NoteTypeValue::Half => NoteEngraving::Half,
        mxd::NoteTypeValue::Quarter => NoteEngraving::Quarter,
        mxd::NoteTypeValue::Eighth => NoteEngraving::Eighth,
        mxd::NoteTypeValue::Sixteenth => NoteEngraving::Sixteenth,
        mxd::NoteTypeValue::ThirtySecond => NoteEngraving::ThirtySecond,
        mxd::NoteTypeValue::SixtyFourth => NoteEngraving::SixtyFourth,
        mxd::NoteTypeValue::OneHundredTwentyEighth => NoteEngraving::OneTwentyEighth,
        _ => NoteEngraving::Quarter,
    }
}

fn is_dotted(class: NoteEngraving) -> bool {
    matches!(
        class,
        NoteEngraving::HalfDotted
            | NoteEngraving::QuarterDotted
            | NoteEngraving::EithDotted
            | NoteEngraving::SixteenthDotted
            | NoteEngraving::ThirtySecondDotted
    )
}

fn dot_engraving(base: NoteEngraving) -> NoteEngraving {
    match base {
        NoteEngraving::Half => NoteEngraving::HalfDotted,
        NoteEngraving::Quarter => NoteEngraving::QuarterDotted,
        NoteEngraving::Eighth => NoteEngraving::EithDotted,
        NoteEngraving::Sixteenth => NoteEngraving::SixteenthDotted,
        NoteEngraving::ThirtySecond => NoteEngraving::ThirtySecondDotted,
        other => other,
    }
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

fn metadata_value(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn score_credit_words(credit: &mx::Credit) -> Option<String> {
    match &credit.content.credit {
        mx::CreditSubcontents::Text(contents) => contents
            .credit_words
            .as_ref()
            .map(|credit_words| credit_words.content.as_str())
            .into_iter()
            .chain(contents.additional.iter().filter_map(|entry| {
                entry
                    .credit_words
                    .as_ref()
                    .map(|credit_words| credit_words.content.as_str())
            }))
            .find_map(|text| metadata_value(Some(text))),
        mx::CreditSubcontents::Image(_) => None,
    }
}

fn score_credit_value(score: &mx::ScorePartwise, credit_type: &str) -> Option<String> {
    score.content.credit.iter().find_map(|credit| {
        credit
            .content
            .credit_type
            .iter()
            .any(|kind| kind.content.trim().eq_ignore_ascii_case(credit_type))
            .then(|| score_credit_words(credit))
            .flatten()
    })
}

fn metadata_credit(credit_type: &str, text: &str) -> mx::Credit {
    mx::Credit {
        attributes: mx::CreditAttributes::default(),
        content: mx::CreditContents {
            credit_type: vec![mx::CreditType {
                attributes: (),
                content: credit_type.to_string(),
            }],
            link: vec![],
            bookmark: vec![],
            credit: mx::CreditSubcontents::Text(mx::CreditTextContents {
                credit_words: Some(mx::CreditWords {
                    attributes: mx::CreditWordsAttributes::default(),
                    content: text.to_string(),
                }),
                credit_symbol: None,
                additional: vec![],
            }),
        },
    }
}

fn score_misc_field(identification: &mx::Identification, name: &str) -> Option<String> {
    identification
        .content
        .miscellaneous
        .as_ref()?
        .content
        .miscellaneous_field
        .iter()
        .find_map(|field| {
            field
                .attributes
                .name
                .0
                .trim()
                .eq_ignore_ascii_case(name)
                .then(|| metadata_value(Some(field.content.as_str())))
                .flatten()
        })
}

fn parse_instrument_name(name: &str) -> Instrument {
    match name.trim().to_ascii_lowercase().as_str() {
        "piano" => Instrument::Piano,
        "guitar" => Instrument::Guitar,
        "bass guitar" => Instrument::BassGuitar,
        "drums" => Instrument::Drums,
        "violin" => Instrument::Violin,
        "viola" => Instrument::Viola,
        "cello" => Instrument::Cello,
        "double bass" => Instrument::DoubleBass,
        "trumpet" => Instrument::Trumpet,
        "saxophone" => Instrument::Saxophone,
        "flute" => Instrument::Flute,
        "oboe" => Instrument::Oboe,
        "clarinet" => Instrument::Clarinet,
        "banjo" => Instrument::Banjo,
        _ => Instrument::Piano,
    }
}

fn parse_staff_number(staff: Option<&mx::Staff>) -> Option<usize> {
    staff.map(|s| (*s.content).min(u32::from(u8::MAX)) as usize)
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
        (ChordQuality::Dominant, Some(7)) => mxd::KindValue::Dominant,
        (ChordQuality::Dominant, Some(9)) => mxd::KindValue::DominantNinth,
        (ChordQuality::Dominant, Some(11)) => mxd::KindValue::Dominant11th,
        (ChordQuality::Dominant, Some(13)) => mxd::KindValue::Dominant13th,
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
        Dominant => (ChordQuality::Dominant, Some(7)),
        DominantNinth => (ChordQuality::Dominant, Some(9)),
        Dominant11th => (ChordQuality::Dominant, Some(11)),
        Dominant13th => (ChordQuality::Dominant, Some(13)),
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

fn divs_to_note_type(divs: u32, dpq: u32) -> (mxd::NoteTypeValue, bool) {
    if dpq == 0 {
        return (mxd::NoteTypeValue::Quarter, false);
    }
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

fn fill_rests(gap_divs: u32, dpq: u32, staff_num: Option<u8>, voice: &str) -> Vec<mx::Note> {
    if dpq == 0 || gap_divs == 0 {
        return vec![];
    }
    let mut remaining = gap_divs;
    let mut notes = vec![];
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
    note_divs: u32,
    staff_num: Option<u8>,
    voice: &str,
    is_chord_tone: bool,
) -> mx::Note {
    let (type_val, dotted) = (
        duration_class_to_type_value(note.engraving),
        is_dotted(note.engraving),
    );
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
                chord: if is_chord_tone {
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
                    content: mxd::PositiveDivisions(note_divs),
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

#[derive(Clone)]
struct PositionedNote {
    start: u32,
    note: NotePlayed,
}

#[derive(Clone)]
struct ChordGroup {
    start: u32,
    duration: u32,
    notes: Vec<NotePlayed>,
}

fn measure_written_voices(measure: &Measure) -> Vec<(usize, Vec<ChordGroup>)> {
    let mut logical_voice_order: Vec<usize> = Vec::new();
    let mut logical_voice_events: Vec<Vec<PositionedNote>> = Vec::new();

    for (storage_voice_idx, voice_notes) in measure.notes.iter().enumerate() {
        let logical_voice = voice_notes
            .iter()
            .find(|note| !note.is_rest())
            .map(|note| note.voice)
            .unwrap_or(storage_voice_idx);

        let logical_voice_idx = if let Some(existing_idx) =
            logical_voice_order.iter().position(|v| *v == logical_voice)
        {
            existing_idx
        } else {
            logical_voice_order.push(logical_voice);
            logical_voice_events.push(Vec::new());
            logical_voice_order.len() - 1
        };

        let mut pos = 0_u32;
        for note in voice_notes {
            if !note.is_rest() {
                logical_voice_events[logical_voice_idx].push(PositionedNote {
                    start: pos,
                    note: note.clone(),
                });
            }
            pos += u32::from(note.duration);
        }
    }

    logical_voice_order
        .into_iter()
        .zip(logical_voice_events)
        .filter_map(|(logical_voice, mut events)| {
            if events.is_empty() {
                return None;
            }

            events.sort_by_key(|event| event.start);

            let mut groups = Vec::new();
            let mut idx = 0;
            while idx < events.len() {
                let start = events[idx].start;
                let mut duration = 0_u32;
                let mut notes = Vec::new();

                while idx < events.len() && events[idx].start == start {
                    duration = duration.max(u32::from(events[idx].note.duration));
                    notes.push(events[idx].note.clone());
                    idx += 1;
                }

                groups.push(ChordGroup {
                    start,
                    duration,
                    notes,
                });
            }

            Some((logical_voice, groups))
        })
        .collect()
}

#[derive(Clone)]
struct ParsedMeasureNote {
    start: u32,
    note: NotePlayed,
}

fn add_parsed_voice(
    voice_order: &mut Vec<String>,
    parsed_voice_events: &mut Vec<Vec<ParsedMeasureNote>>,
    voice_name: &str,
) -> usize {
    if let Some(idx) = voice_order
        .iter()
        .position(|existing| existing == voice_name)
    {
        idx
    } else {
        voice_order.push(voice_name.to_string());
        parsed_voice_events.push(Vec::new());
        voice_order.len() - 1
    }
}

fn rest_note_for_gap(
    duration: u16,
    voice: usize,
    staff: Option<usize>,
    divisions: u16,
) -> NotePlayed {
    NotePlayed {
        note: Note::new(NoteLetter::C, None, 0),
        engraving: NoteEngraving::from_duration_ticks(duration, divisions),
        duration,
        amplitude: 0.0,
        staff,
        voice,
    }
}

fn build_storage_voices_from_parsed(
    mut parsed_voice_events: Vec<Vec<ParsedMeasureNote>>,
    divisions: u16,
) -> Vec<Vec<NotePlayed>> {
    let mut voice_data = Vec::new();

    for (logical_voice, events) in parsed_voice_events.iter_mut().enumerate() {
        events.sort_by_key(|event| event.start);

        let mut stream_ends: Vec<u32> = Vec::new();
        let mut streams: Vec<Vec<ParsedMeasureNote>> = Vec::new();

        for event in events.iter().cloned() {
            if let Some(stream_idx) = stream_ends.iter().position(|end| event.start >= *end) {
                stream_ends[stream_idx] = event.start + u32::from(event.note.duration);
                streams[stream_idx].push(event);
            } else {
                stream_ends.push(event.start + u32::from(event.note.duration));
                streams.push(vec![event]);
            }
        }

        for stream in streams {
            let mut storage_voice = Vec::new();
            let mut cursor = 0_u32;

            for event in stream {
                if event.start > cursor {
                    let gap = (event.start - cursor).min(u32::from(u16::MAX)) as u16;
                    storage_voice.push(rest_note_for_gap(
                        gap,
                        logical_voice,
                        event.note.staff,
                        divisions,
                    ));
                }

                cursor = event.start + u32::from(event.note.duration);
                storage_voice.push(event.note);
            }

            if !storage_voice.is_empty() {
                voice_data.push(storage_voice);
            }
        }
    }

    voice_data
}

// --- Attributes builder ---

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

// --- Score building ---

fn composition_to_score(comp: &Composition) -> mx::ScorePartwise {
    let title = metadata_value(comp.metadata.title.as_deref());
    let subtitle = metadata_value(comp.metadata.subtitle.as_deref());
    let composer = metadata_value(comp.metadata.composer.as_deref());
    let copyright = metadata_value(comp.metadata.copyright.as_deref());

    let part_list = mx::PartList {
        attributes: (),
        content: mx::PartListContents {
            content: comp
                .measures_by_part
                .iter()
                .enumerate()
                .map(|(part_idx, (instr, _))| {
                    let part_id = format!("P{}", part_idx + 1);
                    mx::PartListElement::ScorePart(mx::ScorePart {
                        attributes: mx::ScorePartAttributes {
                            id: mxd::Id(part_id),
                        },
                        content: mx::ScorePartContents {
                            identification: None,
                            part_link: vec![],
                            part_name: mx::PartName {
                                attributes: mx::PartNameAttributes::default(),
                                content: instrument_name(*instr),
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
                    })
                })
                .collect(),
        },
    };

    let parts = comp
        .measures_by_part
        .iter()
        .enumerate()
        .map(|(part_idx, (instr, measures))| {
            let grand = instrument_has_grand_staff(*instr)
                || measures
                    .iter()
                    .any(|measure| measure.staves.contains(&Staff::Grand));
            let mut part_content: Vec<mx::PartElement> = vec![];
            let mut last_key: Option<Key> = None;
            let mut last_ts: Option<TimeSignature> = None;
            let mut last_tempo: Option<u16> = None;
            let mut last_divisions: Option<u16> = None;

            for (mi, measure) in measures.iter().enumerate() {
                let dpq = u32::from(measure.divisions);
                let ts = measure.time_signature;
                let mut measure_content: Vec<mx::MeasureElement> = vec![];

                if mi == 0
                    || last_key != Some(measure.key)
                    || last_ts != Some(ts)
                    || last_divisions != Some(measure.divisions)
                {
                    measure_content.push(mx::MeasureElement::Attributes(make_measure_attrs(
                        measure.key,
                        &ts,
                        *instr,
                        dpq,
                    )));
                }
                last_key = Some(measure.key);
                last_ts = Some(ts);
                last_divisions = Some(measure.divisions);

                if measure.tempo > 0 && Some(measure.tempo) != last_tempo {
                    measure_content.push(mx::MeasureElement::Sound(mx::Sound {
                        attributes: mx::SoundAttributes {
                            tempo: Some(mxd::NonNegativeDecimal(f64::from(measure.tempo))),
                            ..Default::default()
                        },
                        content: Default::default(),
                    }));
                    last_tempo = Some(measure.tempo);
                }

                if let Some(ref chord) = measure.chord {
                    measure_content.push(mx::MeasureElement::Harmony(chord_to_harmony(chord)));
                }

                let measure_total_divs = measure.total_divisions();
                let written_voices = measure_written_voices(measure);

                if written_voices.is_empty() {
                    let staff_num = if grand { Some(1u8) } else { None };
                    for rest in fill_rests(measure_total_divs, dpq, staff_num, "1") {
                        measure_content.push(mx::MeasureElement::Note(rest));
                    }
                } else {
                    for (written_voice_idx, (logical_voice, chord_groups)) in
                        written_voices.iter().enumerate()
                    {
                        if written_voice_idx > 0 {
                            measure_content.push(mx::MeasureElement::Backup(mx::Backup {
                                attributes: (),
                                content: mx::BackupContents {
                                    duration: mx::Duration {
                                        attributes: (),
                                        content: mxd::PositiveDivisions(measure_total_divs),
                                    },
                                    footnote: None,
                                    level: None,
                                },
                            }));
                        }

                        let voice_label = (logical_voice + 1).to_string();
                        let staff_num = if grand {
                            chord_groups
                                .iter()
                                .flat_map(|group| group.notes.iter())
                                .find_map(|note| note.staff)
                                .map(|staff| staff as u8)
                                .or(Some((written_voice_idx.min(1) + 1) as u8))
                        } else {
                            None
                        };

                        let mut cursor = 0_u32;
                        for group in chord_groups {
                            if group.start > cursor {
                                for rest in
                                    fill_rests(group.start - cursor, dpq, staff_num, &voice_label)
                                {
                                    measure_content.push(mx::MeasureElement::Note(rest));
                                }
                            }

                            for (note_idx, note) in group.notes.iter().enumerate() {
                                measure_content.push(mx::MeasureElement::Note(make_pitch_note(
                                    note,
                                    u32::from(note.duration),
                                    staff_num,
                                    &voice_label,
                                    note_idx > 0,
                                )));
                            }

                            cursor = group.start + group.duration;
                        }

                        if cursor < measure_total_divs {
                            for rest in fill_rests(
                                measure_total_divs - cursor,
                                dpq,
                                staff_num,
                                &voice_label,
                            ) {
                                measure_content.push(mx::MeasureElement::Note(rest));
                            }
                        }
                    }
                }

                part_content.push(mx::PartElement::Measure(mx::Measure {
                    attributes: mx::MeasureAttributes {
                        number: mxd::Token((mi + 1).to_string()),
                        id: None,
                        implicit: None,
                        non_controlling: None,
                        text: None,
                        width: None,
                    },
                    content: measure_content,
                }));
            }

            mx::Part {
                attributes: mx::PartAttributes {
                    id: mxd::IdRef(format!("P{}", part_idx + 1)),
                },
                content: part_content,
            }
        })
        .collect();

    mx::ScorePartwise {
        attributes: mx::ScorePartwiseAttributes {
            version: Some(mxd::Token("4.0".to_string())),
        },
        content: mx::ScorePartwiseContents {
            work: title.as_ref().map(|title| mx::Work {
                attributes: (),
                content: mx::WorkContents {
                    work_number: None,
                    work_title: Some(mx::WorkTitle {
                        attributes: (),
                        content: title.clone(),
                    }),
                    opus: None,
                },
            }),
            movement_number: None,
            movement_title: title.as_ref().map(|title| mx::MovementTitle {
                attributes: (),
                content: title.clone(),
            }),
            identification: (composer.is_some() || copyright.is_some() || subtitle.is_some()).then(
                || mx::Identification {
                    attributes: (),
                    content: mx::IdentificationContents {
                        creator: composer
                            .iter()
                            .map(|composer| mx::Creator {
                                attributes: mx::CreatorAttributes {
                                    r#type: Some(mxd::Token("composer".to_string())),
                                },
                                content: composer.clone(),
                            })
                            .collect(),
                        rights: copyright
                            .iter()
                            .map(|copyright| mx::Rights {
                                attributes: mx::RightsAttributes { r#type: None },
                                content: copyright.clone(),
                            })
                            .collect(),
                        encoding: None,
                        source: None,
                        relation: vec![],
                        miscellaneous: subtitle.as_ref().map(|subtitle| mx::Miscellaneous {
                            attributes: (),
                            content: mx::MiscellaneousContents {
                                miscellaneous_field: vec![mx::MiscellaneousField {
                                    attributes: mx::MiscellaneousFieldAttributes {
                                        name: mxd::Token("subtitle".to_string()),
                                    },
                                    content: subtitle.clone(),
                                }],
                            },
                        }),
                    },
                },
            ),
            defaults: None,
            credit: title
                .as_deref()
                .map(|title| metadata_credit("title", title))
                .into_iter()
                .chain(
                    subtitle
                        .as_deref()
                        .map(|subtitle| metadata_credit("subtitle", subtitle)),
                )
                .chain(
                    composer
                        .as_deref()
                        .map(|composer| metadata_credit("composer", composer)),
                )
                .chain(
                    copyright
                        .as_deref()
                        .map(|copyright| metadata_credit("rights", copyright)),
                )
                .collect(),
            part_list,
            part: parts,
        },
    }
}

fn extract_bpm_from_sound(sound: &mx::Sound) -> Option<u16> {
    let bpm = sound.attributes.tempo.as_ref()?.0;
    if bpm > 0.0 && bpm.is_finite() {
        Some(bpm.round().clamp(1.0, f64::from(u16::MAX)) as u16)
    } else {
        None
    }
}

fn score_to_composition(score: &mx::ScorePartwise) -> Composition {
    let mut comp = Composition::new(Temperament::Even, vec![]);
    let title = score
        .content
        .movement_title
        .as_ref()
        .and_then(|title| metadata_value(Some(title.content.as_str())))
        .or_else(|| {
            score.content.work.as_ref().and_then(|work| {
                work.content
                    .work_title
                    .as_ref()
                    .and_then(|title| metadata_value(Some(title.content.as_str())))
            })
        })
        .or_else(|| score_credit_value(score, "title"));
    let subtitle = score
        .content
        .identification
        .as_ref()
        .and_then(|identification| score_misc_field(identification, "subtitle"))
        .or_else(|| score_credit_value(score, "subtitle"));
    let composer = score
        .content
        .identification
        .as_ref()
        .and_then(|identification| {
            identification
                .content
                .creator
                .iter()
                .find_map(|creator| {
                    creator
                        .attributes
                        .r#type
                        .as_ref()
                        .filter(|kind| kind.0.trim().eq_ignore_ascii_case("composer"))
                        .and_then(|_| metadata_value(Some(creator.content.as_str())))
                })
                .or_else(|| {
                    identification
                        .content
                        .creator
                        .iter()
                        .find_map(|creator| metadata_value(Some(creator.content.as_str())))
                })
        })
        .or_else(|| score_credit_value(score, "composer"));
    let copyright = score
        .content
        .identification
        .as_ref()
        .and_then(|identification| {
            identification
                .content
                .rights
                .iter()
                .find_map(|rights| metadata_value(Some(rights.content.as_str())))
        })
        .or_else(|| score_credit_value(score, "rights"));

    comp.metadata = CompMetadata {
        title,
        subtitle,
        composer,
        copyright,
    };
    let part_instruments: Vec<Instrument> = score
        .content
        .part_list
        .content
        .content
        .iter()
        .filter_map(|item| match item {
            mx::PartListElement::ScorePart(score_part) => {
                Some(parse_instrument_name(&score_part.content.part_name.content))
            }
            _ => None,
        })
        .collect();

    for (part_idx, part) in score.content.part.iter().enumerate() {
        let instrument = part_instruments
            .get(part_idx)
            .copied()
            .unwrap_or(Instrument::Piano);
        let mut current_key = Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major);
        let mut current_ts = TimeSignature::new(4, 4);
        let mut current_tempo_bpm: u16 = 120;
        let mut current_divs: u32 = 4;
        let mut voice_order: Vec<String> = Vec::new();
        let mut part_measures = Vec::new();

        for elem in &part.content {
            let mx::PartElement::Measure(mx_measure) = elem else {
                continue;
            };

            let mut parsed_voice_events: Vec<Vec<ParsedMeasureNote>> =
                vec![Vec::new(); voice_order.len()];
            let mut measure_chord: Option<Chord> = None;
            let mut current_pos = 0_u32;
            let mut last_note_start = 0_u32;

            for mel in &mx_measure.content {
                match mel {
                    mx::MeasureElement::Attributes(attrs) => {
                        if let Some(d) = &attrs.content.divisions {
                            current_divs = *d.content;
                        }
                        if let Some(key_el) = attrs.content.key.first() {
                            if let mx::KeyContents::Explicit(exp) = &key_el.content {
                                let fifths = *exp.fifths.content;
                                let mode = match &exp.mode {
                                    Some(m) if m.content == mxd::Mode::Minor => MajorMinor::Minor,
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
                    mx::MeasureElement::Direction(dir) => {
                        if let Some(sound) = &dir.content.sound {
                            if let Some(bpm) = extract_bpm_from_sound(sound) {
                                current_tempo_bpm = bpm;
                            }
                        }
                    }
                    mx::MeasureElement::Sound(sound) => {
                        if let Some(bpm) = extract_bpm_from_sound(sound) {
                            current_tempo_bpm = bpm;
                        }
                    }
                    mx::MeasureElement::Harmony(h) => {
                        measure_chord = harmony_to_chord(h);
                    }
                    mx::MeasureElement::Backup(backup) => {
                        current_pos = current_pos.saturating_sub(*backup.content.duration.content);
                    }
                    mx::MeasureElement::Forward(forward) => {
                        current_pos = current_pos.saturating_add(*forward.content.duration.content);
                    }
                    mx::MeasureElement::Note(note) => {
                        if let mx::NoteType::Normal(info) = &note.content.info {
                            let is_chord = info.chord.is_some();
                            let dur_raw = (*info.duration.content).min(u32::from(u16::MAX)) as u16;
                            let start = if is_chord {
                                last_note_start
                            } else {
                                current_pos
                            };

                            let voice_str = note
                                .content
                                .voice
                                .as_ref()
                                .map(|v| v.content.clone())
                                .unwrap_or_else(|| "1".to_string());

                            let voice_idx = add_parsed_voice(
                                &mut voice_order,
                                &mut parsed_voice_events,
                                &voice_str,
                            );

                            let engraving = if let Some(type_el) = &note.content.r#type {
                                let base = type_value_to_duration_class(&type_el.content);
                                if !note.content.dot.is_empty() {
                                    dot_engraving(base)
                                } else {
                                    base
                                }
                            } else {
                                NoteEngraving::from_duration_ticks(
                                    dur_raw,
                                    current_divs.min(u32::from(u16::MAX)) as u16,
                                )
                            };

                            let staff = parse_staff_number(note.content.staff.as_ref());

                            match &info.audible {
                                mx::AudibleType::Pitch(pitch) => {
                                    let letter = step_to_note_letter(&pitch.content.step.content);
                                    let octave = *pitch.content.octave.content;
                                    let sf = alter_to_sharp_flat(pitch.content.alter.as_ref());
                                    parsed_voice_events[voice_idx].push(ParsedMeasureNote {
                                        start,
                                        note: NotePlayed {
                                            note: Note::new(letter, sf, octave),
                                            engraving,
                                            duration: dur_raw,
                                            amplitude: 0.8,
                                            staff,
                                            voice: voice_idx,
                                        },
                                    });
                                }
                                mx::AudibleType::Rest(_) => {
                                    let _ = (engraving, staff);
                                }
                                mx::AudibleType::Unpitched(_) => {}
                            }

                            if !is_chord {
                                last_note_start = start;
                                current_pos = current_pos.saturating_add(u32::from(dur_raw));
                            }
                        }
                    }
                    _ => {}
                }
            }

            let mut meas = Measure::new(current_key, current_ts, measure_chord, current_tempo_bpm);
            meas.divisions = current_divs.min(u32::from(u16::MAX)) as u16;
            meas.notes = build_storage_voices_from_parsed(parsed_voice_events, meas.divisions);
            let has_staff2 = meas.notes.iter().flatten().any(|n| n.staff == Some(2));
            if has_staff2 {
                meas.staves = vec![Staff::Grand];
            } else {
                meas.staves = vec![Staff::Treble];
            }
            part_measures.push(meas);
        }

        comp.measures_by_part.push((instrument, part_measures));
    }

    comp
}

// --- Public API ---

pub fn write_musicxml(comp: &Composition, format: MusicXmlFormat, path: &Path) -> io::Result<()> {
    let score = composition_to_score(comp);
    match format {
        MusicXmlFormat::Raw => {
            let raw = musicxml::write_partwise_score_data(&score, false, false)
                .map_err(io::Error::other)?;
            let xml = String::from_utf8(raw)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            fs::write(path, normalize_raw_musicxml_doctype(xml))
        }
        MusicXmlFormat::Compressed => {
            let path_str = path
                .to_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF-8 path"))?;
            musicxml::write_partwise_score(path_str, &score, true, false).map_err(io::Error::other)
        }
    }
}

pub fn read_musicxml(path: &Path) -> io::Result<Composition> {
    let path_str = path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF-8 path"))?;
    let score = musicxml::read_score_partwise(path_str)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(score_to_composition(&score))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instrument::Instrument, note::Note};

    fn test_key() -> Key {
        Key::new(NoteLetter::C, SharpFlat::Natural, MajorMinor::Major)
    }

    #[test]
    fn musicxml_reader_honors_staff_voice_and_tempo() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="4.0">
  <part-list>
    <score-part id="P1">
      <part-name>Piano</part-name>
    </score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <attributes>
        <divisions>4</divisions>
        <key><fifths>0</fifths></key>
        <time><beats>4</beats><beat-type>4</beat-type></time>
        <staves>2</staves>
      </attributes>
      <sound tempo="90"/>
      <note>
        <pitch><step>C</step><octave>5</octave></pitch>
        <duration>4</duration>
        <voice>1</voice>
        <type>quarter</type>
        <staff>1</staff>
      </note>
      <backup><duration>4</duration></backup>
      <note>
        <pitch><step>C</step><octave>3</octave></pitch>
        <duration>4</duration>
        <voice>5</voice>
        <type>quarter</type>
        <staff>2</staff>
      </note>
    </measure>
  </part>
</score-partwise>"#;

        let score = musicxml::read_score_data_partwise(xml.as_bytes().to_vec()).unwrap();
        let composition = score_to_composition(&score);

        assert_eq!(composition.measures_by_part.len(), 1);
        let meas = &composition.measures_by_part[0].1[0];
        assert_eq!(meas.tempo, 90);
        assert_eq!(meas.divisions, 4);
        // Two voices: "1" and "5"
        assert_eq!(meas.notes.len(), 2);
        let treble = meas.notes[0].iter().find(|n| n.staff == Some(1)).unwrap();
        assert_eq!(treble.note.octave, 5);
        let bass = meas.notes[1].iter().find(|n| n.staff == Some(2)).unwrap();
        assert_eq!(bass.note.octave, 3);
    }

    #[test]
    fn musicxml_writer_emits_backup_between_voices() {
        let key = test_key();
        let mut comp = Composition::new(Temperament::Even, vec![Instrument::Piano]);
        let mut measure = Measure::new(key, TimeSignature::new(4, 4), None, 100);
        measure.divisions = 4;
        let half_dur = NoteEngraving::Half.to_duration_ticks(4);
        measure.notes = vec![
            vec![NotePlayed {
                note: Note::new(NoteLetter::C, Some(SharpFlat::Natural), 5),
                engraving: NoteEngraving::Half,
                duration: half_dur,
                amplitude: 0.8,
                staff: Some(1),
                voice: 0,
            }],
            vec![NotePlayed {
                note: Note::new(NoteLetter::C, Some(SharpFlat::Natural), 3),
                engraving: NoteEngraving::Half,
                duration: half_dur,
                amplitude: 0.8,
                staff: Some(2),
                voice: 1,
            }],
        ];
        comp.measures_by_part[0].1.push(measure);

        let score = composition_to_score(&comp);
        let mx::PartElement::Measure(m) = &score.content.part[0].content[0] else {
            panic!("expected measure");
        };

        let backup_count = m
            .content
            .iter()
            .filter(|e| matches!(e, mx::MeasureElement::Backup(_)))
            .count();
        assert_eq!(backup_count, 1);
    }

    #[test]
    fn musicxml_round_trip_preserves_same_voice_chords() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="4.0">
  <part-list>
    <score-part id="P1">
      <part-name>Piano</part-name>
    </score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <attributes>
        <divisions>12</divisions>
        <key><fifths>-3</fifths><mode>minor</mode></key>
        <time><beats>6</beats><beat-type>8</beat-type></time>
        <staves>2</staves>
      </attributes>
      <note>
        <pitch><step>A</step><alter>-1</alter><octave>4</octave></pitch>
        <duration>12</duration>
        <voice>1</voice>
        <type>quarter</type>
        <staff>1</staff>
      </note>
      <note>
        <chord/>
        <pitch><step>C</step><octave>5</octave></pitch>
        <duration>12</duration>
        <voice>1</voice>
        <type>quarter</type>
        <staff>1</staff>
      </note>
      <note>
        <chord/>
        <pitch><step>G</step><octave>5</octave></pitch>
        <duration>12</duration>
        <voice>1</voice>
        <type>quarter</type>
        <staff>1</staff>
      </note>
    </measure>
  </part>
</score-partwise>"#;

        let score = musicxml::read_score_data_partwise(xml.as_bytes().to_vec()).unwrap();
        let composition = score_to_composition(&score);
        let round_tripped = composition_to_score(&composition);

        let mx::PartElement::Measure(measure) = &round_tripped.content.part[0].content[0] else {
            panic!("expected measure");
        };

        let note_elements: Vec<&mx::Note> = measure
            .content
            .iter()
            .filter_map(|element| match element {
                mx::MeasureElement::Note(note) => Some(note),
                _ => None,
            })
            .collect();
        let pitched_note_elements: Vec<&mx::Note> = note_elements
            .into_iter()
            .filter(|note| {
                matches!(
                    &note.content.info,
                    mx::NoteType::Normal(info) if matches!(info.audible, mx::AudibleType::Pitch(_))
                )
            })
            .collect();
        assert_eq!(pitched_note_elements.len(), 3);

        let chord_flags: Vec<bool> = pitched_note_elements
            .iter()
            .map(|note| match &note.content.info {
                mx::NoteType::Normal(info) => info.chord.is_some(),
                _ => false,
            })
            .collect();
        assert_eq!(chord_flags, vec![false, true, true]);

        let backup_count = measure
            .content
            .iter()
            .filter(|element| matches!(element, mx::MeasureElement::Backup(_)))
            .count();
        assert_eq!(backup_count, 0);
    }

    #[test]
    fn raw_musicxml_doctype_is_upgraded_to_4_0() {
        let comp = Composition::new(Temperament::Even, vec![Instrument::Piano]);
        let score = composition_to_score(&comp);
        let raw = musicxml::write_partwise_score_data(&score, false, false).unwrap();
        let normalized = normalize_raw_musicxml_doctype(String::from_utf8(raw).unwrap());

        assert!(normalized.contains(MUSICXML_4_PARTWISE_DOCTYPE));
        assert!(!normalized.contains(MUSICXML_3_PARTWISE_DOCTYPE));
    }

    #[test]
    fn musicxml_round_trip_preserves_score_metadata() {
        let key = test_key();
        let mut comp = Composition::new(Temperament::Even, vec![Instrument::Piano]);
        comp.metadata.title = Some("Starlight".to_string());
        comp.metadata.subtitle = Some("For Quiet Evenings".to_string());
        comp.metadata.composer = Some("A. Example".to_string());
        comp.metadata.copyright = Some("Copyright 2026 A. Example".to_string());
        comp.measures_by_part[0]
            .1
            .push(Measure::new(key, TimeSignature::new(4, 4), None, 96));

        let score = composition_to_score(&comp);
        let raw = musicxml::write_partwise_score_data(&score, false, false).unwrap();
        let parsed = musicxml::read_score_data_partwise(raw).unwrap();
        let round_tripped = score_to_composition(&parsed);

        assert_eq!(round_tripped.metadata.title.as_deref(), Some("Starlight"));
        assert_eq!(
            round_tripped.metadata.subtitle.as_deref(),
            Some("For Quiet Evenings")
        );
        assert_eq!(
            round_tripped.metadata.composer.as_deref(),
            Some("A. Example")
        );
        assert_eq!(
            round_tripped.metadata.copyright.as_deref(),
            Some("Copyright 2026 A. Example")
        );
    }
}
