//! Experimenting in generating music.

use rand::RngExt;

use crate::{
    composition::NotesStartingThisTick,
    key_scale::SharpFlat,
    measure::{ChordProgression, TimeSignature},
    note::{Note, NoteDuration, NoteDurationClass, NotePlayed},
};
use crate::measure::Measure;

const AMPLITUDE: f32 = 0.2;

fn beat_duration(denominator: u8) -> NoteDurationClass {
    match denominator {
        1 => NoteDurationClass::Whole,
        2 => NoteDurationClass::Half,
        4 => NoteDurationClass::Quarter,
        8 => NoteDurationClass::Eighth,
        16 => NoteDurationClass::Sixteenth,
        32 => NoteDurationClass::ThirtySecond,
        64 => NoteDurationClass::SixtyFourth,
        128 => NoteDurationClass::OneTwentyEighth,
        _ => NoteDurationClass::Quarter,
    }
}

/// One beat-note of the chord's root per beat, filling the full measure.
/// E.g. 4/4 → 4 quarter notes; 6/8 → 6 eighth notes.
pub fn make_bassline_roots(
    // prog: &ChordProgression,
    measures: &[Measure],
) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();
    // let octaves = [2u8, 3, 4];
    let octaves = [3];

    let mut rng = rand::rng();
    let duration = NoteDuration::Traditional(beat_duration(time_sig.denominator));

    for meas in &measures {
        let Some(chord) = &meas.chord else {
            eprintln!("Error: Missing a chord when creating a bassline from chords");
            return Vec::new();
        };

        let octave = octaves[rng.random_range(0..octaves.len())];
        let note = Note::new(chord.root, Some(SharpFlat::Natural), octave);

        for _ in 0..meas.time_sig.numerator {
            res.push(NotesStartingThisTick {
                notes: vec![NotePlayed {
                    note: note.clone(),
                    duration,
                    amplitude: AMPLITUDE,
                }],
            });
        }
    }

    res
}

/// Cycles through the chord's notes ascending, one per beat, filling the measure.
/// E.g. 4/4 with a triad → root, 3rd, 5th, root.
pub fn make_bassline_ascending(
    // prog: &ChordProgression,
    measures: &[Measure],
    time_sig: TimeSignature,
) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();
    let amplitude = 0.8;
    let duration = NoteDuration::Traditional(beat_duration(time_sig.denominator));

    for (subset_i, reps) in &prog.sets {
        let subset = &prog.subsets[*subset_i];
        for _ in 0..*reps {
            for chord in subset {
                let notes = chord.notes();
                for beat in 0..time_sig.numerator as usize {
                    res.push(NotesStartingThisTick {
                        notes: vec![NotePlayed {
                            note: notes[beat % notes.len()].clone(),
                            duration,
                            amplitude: AMPLITUDE,
                        }],
                    });
                }
            }
        }
    }

    res
}

/// Random chord notes, one per beat. With `start_at_root`, the first beat of each chord is always the root.
pub fn make_bassline_random(
    // prog: &ChordProgression,
    measures: &[Measure],
    time_sig: TimeSignature,
    start_at_root: bool,
) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();
    let mut rng = rand::rng();
    let duration = NoteDuration::Traditional(beat_duration(time_sig.denominator));

    for (subset_i, reps) in &prog.sets {
        let subset = &prog.subsets[*subset_i];
        for _ in 0..*reps {
            for chord in subset {
                let notes = chord.notes();
                for beat in 0..time_sig.numerator as usize {
                    let note = if start_at_root && beat == 0 {
                        notes[0].clone()
                    } else {
                        notes[rng.random_range(0..notes.len())].clone()
                    };
                    res.push(NotesStartingThisTick {
                        notes: vec![NotePlayed {
                            note,
                            duration,
                            amplitude: AMPLITUDE,
                        }],
                    });
                }
            }
        }
    }

    res
}
