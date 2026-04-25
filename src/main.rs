use std::path::Path;

use chord::{Chord, ChordQuality::*};
use egui::Ui;
use key_scale::{Key, MajorMinor, SharpFlat};

use crate::{
    chord::{Inversion, prog_1451, prog_1564, prog_1645, prog_pachabel},
    composition::{Composition, NotesStartingThisTick},
    gui::{WINDOW_HEIGHT, WINDOW_WIDTH},
    instrument::Instrument,
    make_bass_music::make_bassline_random,
    measure::{Measure, TimeSignature},
    music_xml::MusicXmlFormat,
    note::{Note, NoteLetter},
    overtones::Temperament,
};

mod chord;
mod composition;
mod composition_arch;
mod decomposition;
mod generation;
mod gui;
mod instrument;
mod key_scale;
mod make_bass_music;
mod measure;
mod midi;
mod music_xml;
mod note;
mod overtones;
mod player;
//

// pub struct NotePlayed {
//     /// hz
//     pub pitch: f32,
//     /// seconds
//     pub duration: f32,
// }

#[derive(Default)]
pub struct State {
    pub compositions: Vec<Composition>,
}

impl eframe::App for State {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        gui::draw(self, ui.ctx());
    }
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
        staff: None,
        voice: None,
    };

    let new_q = |letter: NoteLetter, octave: u8| NotePlayed {
        note: Note::new(letter, None, octave),
        duration: qu,
        amplitude,
        staff: None,
        voice: None,
    };

    let new_q_dot = |letter: NoteLetter, octave: u8| NotePlayed {
        note: Note::new(letter, None, octave),
        duration: NoteDuration::Traditional(QuarterDotted),
        amplitude,
        staff: None,
        voice: None,
    };

    let new_h = |letter: NoteLetter, octave: u8| NotePlayed {
        note: Note::new(letter, None, octave),
        duration: ha,
        amplitude,
        staff: None,
        voice: None,
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

    for note in notes_m0 {
        res.notes_by_tick.push(note);
    }

    res
}

fn make_test_bassline() -> Composition {
    use crate::{
        key_scale::{MajorMinor, SharpFlat::*},
        note::NoteLetter::*,
    };

    let root_note = Note::new(C, None, 3);

    let key = Key::new(root_note.letter, Natural, MajorMinor::Major);
    let sig = TimeSignature::new(4, 4);

    let fourth = root_note.add_interval(5);
    let fifth = root_note.add_interval(7);

    let chord_c = Chord::new(root_note, Major, None, vec![], Inversion::Root);
    let chord_f = Chord::new(fourth, Major, None, vec![], Inversion::Root);
    let chord_g = Chord::new(fifth, Major, None, vec![], Inversion::Root);

    let measures = vec![
        Measure::new(key, sig, Some(chord_c.clone()), 100),
        Measure::new(key, sig, Some(chord_f.clone()), 100),
        Measure::new(key, sig, Some(chord_c.clone()), 100),
        Measure::new(key, sig, Some(chord_c.clone()), 100),
        Measure::new(key, sig, Some(chord_f.clone()), 100),
        Measure::new(key, sig, Some(chord_f.clone()), 100),
        Measure::new(key, sig, Some(chord_c.clone()), 100),
        Measure::new(key, sig, Some(chord_c.clone()), 100),
        Measure::new(key, sig, Some(chord_g.clone()), 100),
        Measure::new(key, sig, Some(chord_f.clone()), 100),
        Measure::new(key, sig, Some(chord_c.clone()), 100),
        Measure::new(key, sig, Some(chord_f.clone()), 100),
    ];

    // let notes = make_bassline_ascending(&measures, 1);
    // let notes = make_bassline_roots(&measures, 1);
    // let notes = make_bassline_ascending(&measures, 1);
    let notes = make_bassline_random(&measures, 1, true);

    let mut comp = Composition::new(
        1,
        100,
        key,
        Temperament::WellTempered(key),
        vec![Instrument::BassGuitar],
    );

    for note in &notes {
        println!("-{note}");
    }

    comp.notes_by_tick = notes;
    comp.measures = measures;

    comp
}

/// Make a simple composition for piano using a chord progression. 4/4 time signature. The right hand of the piano,
/// on the treble clef, will play two half-note chords per measure. (The full chord). The left hand (bass clef)
/// will play `make_random_baseline()` of the chord.
///
/// One measure per chord.
fn make_comp_from_prog(key: Key, chords: &[Chord]) -> Composition {
    use crate::note::{
        NoteDuration,
        NoteDurationClass::{Half, Quarter},
        NotePlayed,
    };

    let sig = TimeSignature::new(4, 4);
    let ticks_per_sixteenth: u32 = 1;
    let ms_per_tick = 100;
    let amplitude = 0.2;
    let half_dur = NoteDuration::Traditional(Half);

    let measures: Vec<Measure> = chords
        .iter()
        .map(|chord| Measure::new(key, sig, Some(chord.clone()), ms_per_tick))
        .collect();

    // Left hand: use chords at octave 3 for bass register
    let bass_measures: Vec<Measure> = chords
        .iter()
        .map(|chord| {
            let bass_root = Note::new(chord.root.letter, chord.root.sharp_flat, 3);
            let bass_chord = Chord::new(
                bass_root,
                chord.quality,
                chord.extension,
                chord.alterations.clone(),
                Inversion::Root,
            );
            Measure::new(key, sig, Some(bass_chord), ms_per_tick)
        })
        .collect();

    let mut notes = make_bassline_random(&bass_measures, ticks_per_sixteenth, true);

    // Tag all bass notes as staff 2 (left hand)
    for tick in &mut notes {
        for note in &mut tick.notes {
            note.staff = Some(2);
        }
    }

    // 4/4: 4 beats × 4 ticks/beat (quarter note = 4 ticks at ticks_per_sixteenth=1)
    let ticks_per_beat = NoteDuration::Traditional(Quarter)
        .get_ticks(ticks_per_sixteenth)
        .unwrap();
    let ticks_per_measure = sig.numerator as u32 * ticks_per_beat;
    let half_ticks = half_dur.get_ticks(ticks_per_sixteenth).unwrap();

    // Right hand: two half-note chord voicings per measure, staff 1
    for (i, chord) in chords.iter().enumerate() {
        let chord_notes: Vec<NotePlayed> = chord
            .notes()
            .into_iter()
            .map(|note| NotePlayed {
                note,
                duration: half_dur,
                amplitude,
                staff: Some(1),
                voice: None,
            })
            .collect();

        let tick_a = i as u32 * ticks_per_measure;
        let tick_b = tick_a + half_ticks;

        notes[tick_a as usize].notes.extend(chord_notes.clone());
        notes[tick_b as usize].notes.extend(chord_notes);
    }

    let mut comp = Composition::new(
        ticks_per_sixteenth,
        ms_per_tick,
        key,
        Temperament::WellTempered(key),
        vec![Instrument::Piano],
    );

    comp.notes_by_tick = notes;
    comp.measures = measures;

    comp
}

fn main() {
    // todo: Placeholders
    // let comp = Composition::from_midi(&Path) {
    //
    // }

    // todo: Placeholders.
    // let comp = Composition::from_musicxml(&Path) {
    //
    // }

    // let prog_0 = prog_1451(Key::new(NoteLetter::G, SharpFlat::Natural, MajorMinor::Major));
    let key = Key::new(NoteLetter::C, SharpFlat::Sharp, MajorMinor::Minor);
    // let prog = prog_pachabel(key);
    let prog = prog_1645(key);

    println!("Prog:");
    for chord in &prog {
        println!("- {chord}")
    }
    let comp = make_comp_from_prog(key, &prog);

    // let comp = make_test_composition();
    // let comp = make_test_bassline();

    let xml_path = Path::new("./demo.musicxml");
    let midi_path = Path::new("./demo.mid");

    comp.save_musicxml(MusicXmlFormat::Raw, &xml_path).unwrap();
    comp.save_midi(&midi_path).unwrap();

    let comp_loaded = Composition::load_musicxml(
        Path::new("C:/Users/the_a/compositions/training_etc/alicia-clair-obscur-expedition-33-main-theme-piano-solo.mxl")
    ).unwrap();
    comp_loaded
        .save_midi(Path::new("./comp_loaded.mid"))
        .unwrap();
    comp_loaded
        .save_musicxml(MusicXmlFormat::Raw, Path::new("./comp_loaded.musicxml"))
        .unwrap();

    println!("\nComp: {comp_loaded}");

    for (i, note) in comp_loaded.notes_by_tick.iter().enumerate() {
        if !i.is_multiple_of(comp_loaded.ticks_per_sixteenth_note as usize * 2) {
            continue; //
        }
        println!(" -Tick {i}: {note}");
    }

    // comp_loaded.play().unwrap();
    // comp.play().unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT]),
        ..Default::default()
    };

    let state = {
        let mut v = State::default();

        v.compositions.push(comp_loaded);

        v
    };

    eframe::run_native("Music", options, Box::new(|_cc| Ok(Box::new(state)))).unwrap();
}
