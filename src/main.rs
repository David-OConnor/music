use key_scale::{Key, MajorMinor, SharpFlat};

use crate::{
    composition::{Composition, NotesStartingThisTick},
    instrument::Instrument,
    make_base_music::make_bassline_roots,
    measure::TimeSignature,
    note::{Note, NoteLetter},
    overtones::Temperament,
};

mod composition;
mod decomposition;
mod generation;
mod instrument;
mod key_scale;
mod make_base_music;
mod measure;
mod note;
mod overtones;
mod player;
mod sheet_music;
//

// pub struct NotePlayed {
//     /// hz
//     pub pitch: f32,
//     /// seconds
//     pub duration: f32,
// }

pub struct State {
    pub compositions: Vec<Composition>,
}

/// We are using this to develop our data structures.
/// The opening of *Alicia* from the Expedition 33 sound track.
pub fn make_test_composition() -> Composition {
    use measure::TimeSignature;
    use note::{NoteDuration, NoteDurationClass::*, NoteLetter::*, NotePlayed};

    let ei = NoteDuration::Traditional(Eighth);
    let qu = NoteDuration::Traditional(Quarter);
    let ha = NoteDuration::Traditional(Half);
    let instruments = vec![
        Instrument::Violin, // Treble clef
        Instrument::BassGuitar,
    ];

    let key = Key::new(C, SharpFlat::Natural, MajorMinor::Minor);
    let ms_per_tick = 340;

    let mut res = Composition::new(
        1,
        ms_per_tick,
        key,
        // Temperament::Even,
        Temperament::WellTempered(key),
        instruments,
    );

    let sig = TimeSignature::new(6, 8);

    let amplitude = 0.2;

    let new_e = |letter: NoteLetter, octave: u8| NotePlayed {
        note: Note::new(letter, None, octave),
        duration: ei,
        amplitude,
    };

    let new_q = |letter: NoteLetter, octave: u8| NotePlayed {
        note: Note::new(letter, None, octave),
        duration: qu,
        amplitude,
    };

    let new_q_dot = |letter: NoteLetter, octave: u8| NotePlayed {
        note: Note::new(letter, None, octave),
        duration: NoteDuration::Traditional(QuarterDotted),
        amplitude,
    };

    let new_h = |letter: NoteLetter, octave: u8| NotePlayed {
        note: Note::new(letter, None, octave),
        duration: ha,
        amplitude,
    };

    // todo: How do we assign an instrument?
    // todo: Currently we could play this sequentially.
    let notes_m0 = vec![
        // M1
        NotesStartingThisTick {
            notes: vec![new_e(C, 3), new_e(C, 6)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(G, 3), new_e(G, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(C, 4), new_h(F, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(D, 4)],
        },
        NotesStartingThisTick {
            notes: vec![new_q(E, 4)],
        },
        NotesStartingThisTick::empty(),
        // Measure 2 ------------
        NotesStartingThisTick {
            notes: vec![new_e(C, 3), new_e(C, 6)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(G, 3), new_e(B, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(D, 4), new_h(E, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(E, 4)],
        },
        NotesStartingThisTick {
            notes: vec![new_q(F, 4)],
        },
        NotesStartingThisTick::empty(),
        // Measure 3 ------------
        NotesStartingThisTick {
            notes: vec![new_e(C, 3), new_e(G, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(G, 3), new_e(F, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(C, 4), new_h(A, 4)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(D, 4)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(E, 4)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(F, 4)],
        },
        // Measure 4 ------------
        NotesStartingThisTick {
            notes: vec![new_e(C, 3), new_e(E, 4), new_e(E, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(G, 3), new_e(D, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(C, 4), new_e(G, 4)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(E, 4), new_e(E, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(D, 4), new_e(D, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(E, 4), new_e(E, 5)],
        },
        // Measure 5 ------------
        NotesStartingThisTick {
            notes: vec![new_e(F, 2), new_q(A, 4), new_q(C, 5), new_q(G, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(C, 3)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(F, 3), new_e(E, 4)],
        },
        NotesStartingThisTick {
            notes: vec![new_e(G, 4), new_q_dot(F, 5)],
        },
        NotesStartingThisTick {
            notes: vec![new_q(A, 4)],
        },
        NotesStartingThisTick::empty(),
    ];

    // let meas_1 = meas_0.clone();
    //
    // let measures = vec![meas_0, meas_1];
    //
    // for m in measures {
    //     res.add_measure(m);
    // }

    for note in notes_m0 {
        res.notes_by_tick.push(note);
    }

    res
}

fn make_test_baseline() -> Composition {
    use crate::{
        make_base_music::make_bassline_ascending,
        measure::ChordProgression,
        note::{Chord, ChordType::*, NoteLetter::*},
    };

    let key = Key::new(C, SharpFlat::Natural, MajorMinor::Minor);

    // i iv V i in C minor, low octave for a bassline
    let prog = ChordProgression {
        subsets: vec![vec![
            Chord::new(C, Major, None, 3),
            Chord::new(F, Major, None, 3),
            Chord::new(C, Major, None, 3),
            Chord::new(C, Major, None, 3),
            Chord::new(F, Major, None, 3),
            Chord::new(F, Major, None, 3),
            Chord::new(C, Major, None, 3),
            Chord::new(C, Major, None, 3),
            Chord::new(G, Major, None, 3),
            Chord::new(F, Major, None, 3),
            Chord::new(C, Major, None, 3),
            Chord::new(F, Major, None, 3),
        ]],
        sets: vec![(0, 2)],
    };

    let sig = TimeSignature::new(4, 4);

    // let notes = make_bassline_ascending(&prog);
    let notes = make_bassline_roots(&prog, sig);

    let mut comp = Composition::new(
        1,
        500,
        key,
        Temperament::WellTempered(key),
        vec![Instrument::BassGuitar],
    );

    for note in &notes {
        println!("-{note}");
    }

    comp.notes_by_tick = notes;
    comp.chord_progression = Some(prog);

    comp
}

fn main() {
    // let comp = make_test_composition();
    let comp = make_test_baseline();

    comp.play().unwrap();
}
