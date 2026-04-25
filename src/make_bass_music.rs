//! Experimenting in generating music.

use rand::RngExt;

use crate::{
    composition::NotesStartingThisTick,
    measure::Measure,
    note::{Note, NoteDurationGeneral, NoteEngraving, NotePlayed},
};

const AMPLITUDE: f32 = 0.2;

fn beat_duration(denominator: u8) -> NoteEngraving {
    match denominator {
        1 => NoteEngraving::Whole,
        2 => NoteEngraving::Half,
        4 => NoteEngraving::Quarter,
        8 => NoteEngraving::Eighth,
        16 => NoteEngraving::Sixteenth,
        32 => NoteEngraving::ThirtySecond,
        64 => NoteEngraving::SixtyFourth,
        128 => NoteEngraving::OneTwentyEighth,
        _ => NoteEngraving::Quarter,
    }
}

fn beat_ticks(denominator: u8, ticks_per_sixteenth: u32) -> u32 {
    let dur = NoteDurationGeneral::Traditional(beat_duration(denominator));
    dur.get_ticks(ticks_per_sixteenth)
        .unwrap_or(ticks_per_sixteenth * 4)
}

/// One beat-note of the chord's root per beat, filling the full measure.
/// E.g. 4/4 → 4 quarter notes; 6/8 → 6 eighth notes.
pub fn make_bassline_roots(
    measures: &[Measure],
    ticks_per_sixteenth: u32,
) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();
    let octaves = [3];
    let mut rng = rand::rng();

    for meas in measures {
        let Some(chord) = &meas.chord else {
            eprintln!("Error: Missing a chord when creating a bassline from chords");
            return Vec::new();
        };

        let ts = meas.time_signature;
        let duration = NoteDurationGeneral::Traditional(beat_duration(ts.denominator));
        let ticks_per_beat = beat_ticks(ts.denominator, ticks_per_sixteenth);
        let octave = octaves[rng.random_range(0..octaves.len())];
        let note = Note::new(chord.root.letter, chord.root.sharp_flat, octave);

        for _ in 0..ts.numerator {
            res.push(NotesStartingThisTick {
                notes: vec![NotePlayed {
                    note: note.clone(),
                    engraving: duration,
                    amplitude: AMPLITUDE,
                    staff: None,
                    voice: None,
                }],
            });
            for _ in 1..ticks_per_beat {
                res.push(NotesStartingThisTick::empty());
            }
        }
    }

    res
}

/// Cycles through the chord's notes ascending, one per beat, filling the measure.
/// E.g. 4/4 with a triad → root, 3rd, 5th, root.
pub fn make_bassline_ascending(
    measures: &[Measure],
    ticks_per_sixteenth: u32,
) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();

    for meas in measures {
        let Some(chord) = &meas.chord else {
            eprintln!("Error: Missing a chord when creating a bassline from chords");
            return Vec::new();
        };

        let ts = meas.time_signature;
        let duration = NoteDurationGeneral::Traditional(beat_duration(ts.denominator));
        let ticks_per_beat = beat_ticks(ts.denominator, ticks_per_sixteenth);
        let notes = chord.notes();

        for beat in 0..ts.numerator as usize {
            res.push(NotesStartingThisTick {
                notes: vec![NotePlayed {
                    note: notes[beat % notes.len()].clone(),
                    engraving: duration,
                    amplitude: AMPLITUDE,
                    staff: None,
                    voice: None,
                }],
            });
            for _ in 1..ticks_per_beat {
                res.push(NotesStartingThisTick::empty());
            }
        }
    }

    res
}

/// Random chord notes, one per beat. With `start_at_root`, the first beat of each chord is always the root.
pub fn make_bassline_random(
    measures: &[Measure],
    ticks_per_sixteenth: u32,
    start_at_root: bool,
) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();
    let mut rng = rand::rng();

    for meas in measures {
        let Some(chord) = &meas.chord else {
            eprintln!("Error: Missing a chord when creating a bassline from chords");
            return Vec::new();
        };

        let ts = meas.time_signature;
        let duration = NoteDurationGeneral::Traditional(beat_duration(ts.denominator));
        let ticks_per_beat = beat_ticks(ts.denominator, ticks_per_sixteenth);
        let notes = chord.notes();

        for beat in 0..ts.numerator as usize {
            let note = if start_at_root && beat == 0 {
                notes[0].clone()
            } else {
                notes[rng.random_range(0..notes.len())].clone()
            };
            res.push(NotesStartingThisTick {
                notes: vec![NotePlayed {
                    note,
                    engraving: duration,
                    amplitude: AMPLITUDE,
                    staff: None,
                    voice: None,
                }],
            });
            for _ in 1..ticks_per_beat {
                res.push(NotesStartingThisTick::empty());
            }
        }
    }

    res
}
