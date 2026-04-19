//! Experimenting in generating music.

use crate::composition::NotesStartingThisTick;
use crate::measure::ChordProgression;
use crate::note::{Note, NoteDuration, NoteDurationClass, NotePlayed};

use rand::Rng;

/// Quarter notes of the chord's root note.
pub fn make_bassline_roots(prog: &ChordProgression) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();

    // Choose root notes etc from these octaves;
    let octaves = [1, 2, 3];
    let amplitude = 1.;

    for (subset_i, reps) in &prog.sets {
        let subset = &prog.subsets[*subset_i];

        for _ in 0..*reps {
            for chord in subset {
                let octave: u8 = Rng::get(octaves); // todo: Syntax.

                res.push(NotesStartingThisTick {
                    notes: vec![
                        NotePlayed {
                            // todo: get sharp_flat.
                            note: Note::new(chord.root, sharp_flat, octave),
                            duration: NoteDuration::Traditional(NoteDurationClass::Quarter),
                            amplitude
                        }
                    ]
                })
            }
        }
    }


    res
}

/// Quarter notes starting at the scale's root each chord change, and incrementing through the chord
/// notes, ascending.
pub fn make_bassline_ascending(prog: &ChordProgression) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();


    res
}

/// Quarter notes of random notes in the chord. Can either make it truly random notes in the chord,
/// or have it start at the root note each time the chord changes.
pub fn make_bassline_random(prog: &ChordProgression, start_at_root: bool) -> Vec<NotesStartingThisTick> {
    let mut res = Vec::new();


    res
}