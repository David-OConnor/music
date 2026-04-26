//! Experimenting in generating music.

use rand::RngExt;

use crate::{
    measure::Measure,
    note::{NoteEngraving, NotePlayed},
};

const AMPLITUDE: f32 = 0.2;

fn beat_engraving(denominator: u8) -> NoteEngraving {
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

fn ensure_voice(measure: &mut Measure, voice_idx: usize) {
    while measure.notes.len() <= voice_idx {
        measure.notes.push(Vec::new());
    }
}

/// One beat-note of the chord's root per beat, filling the full measure.
pub fn make_bassline_roots(measures: &mut [Measure], voice_idx: usize) {
    let octaves = [3usize];
    let mut rng = rand::rng();

    for meas in measures.iter_mut() {
        let Some(chord) = meas.chord.clone() else {
            eprintln!("Error: Missing a chord when creating a bassline from chords");
            return;
        };

        let ts = meas.time_signature;
        let divisions = meas.divisions;
        let eng = beat_engraving(ts.denominator);
        let dur = eng.to_duration_ticks(divisions);
        let octave = octaves[rng.random_range(0..octaves.len())] as u8;
        let note = crate::note::Note::new(chord.root.letter, chord.root.sharp_flat, octave);

        let voice_notes: Vec<NotePlayed> = (0..ts.numerator)
            .map(|_| NotePlayed {
                note: note.clone(),
                engraving: eng,
                duration: dur,
                amplitude: AMPLITUDE,
                staff: None,
                voice: voice_idx,
            })
            .collect();

        ensure_voice(meas, voice_idx);
        meas.notes[voice_idx] = voice_notes;
    }
}

/// Cycles through the chord's notes ascending, one per beat, filling the measure.
pub fn make_bassline_ascending(measures: &mut [Measure], voice_idx: usize) {
    for meas in measures.iter_mut() {
        let Some(chord) = meas.chord.clone() else {
            eprintln!("Error: Missing a chord when creating a bassline from chords");
            return;
        };

        let ts = meas.time_signature;
        let divisions = meas.divisions;
        let eng = beat_engraving(ts.denominator);
        let dur = eng.to_duration_ticks(divisions);
        let notes = chord.notes();

        let voice_notes: Vec<NotePlayed> = (0..ts.numerator as usize)
            .map(|beat| NotePlayed {
                note: notes[beat % notes.len()].clone(),
                engraving: eng,
                duration: dur,
                amplitude: AMPLITUDE,
                staff: None,
                voice: voice_idx,
            })
            .collect();

        ensure_voice(meas, voice_idx);
        meas.notes[voice_idx] = voice_notes;
    }
}

/// Random chord notes, one per beat. With `start_at_root`, the first beat is always the root.
pub fn make_bassline_random(measures: &mut [Measure], voice_idx: usize, start_at_root: bool) {
    let mut rng = rand::rng();

    for meas in measures.iter_mut() {
        let Some(chord) = meas.chord.clone() else {
            eprintln!("Error: Missing a chord when creating a bassline from chords");
            return;
        };

        let ts = meas.time_signature;
        let divisions = meas.divisions;
        let eng = beat_engraving(ts.denominator);
        let dur = eng.to_duration_ticks(divisions);
        let notes = chord.notes();

        let voice_notes: Vec<NotePlayed> = (0..ts.numerator as usize)
            .map(|beat| {
                let note = if start_at_root && beat == 0 {
                    notes[0].clone()
                } else {
                    notes[rng.random_range(0..notes.len())].clone()
                };
                NotePlayed {
                    note,
                    engraving: eng,
                    duration: dur,
                    amplitude: AMPLITUDE,
                    staff: None,
                    voice: voice_idx,
                }
            })
            .collect();

        ensure_voice(meas, voice_idx);
        meas.notes[voice_idx] = voice_notes;
    }
}
