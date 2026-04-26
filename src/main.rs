use std::path::Path;

use chord::{Chord, ChordQuality::*};
use egui::Ui;
use key_scale::{Key, MajorMinor, SharpFlat};

use crate::{
    chord::{Inversion, prog_1645},
    composition::Composition,
    gui::{WINDOW_HEIGHT, WINDOW_WIDTH},
    instrument::Instrument,
    make_bass_music::make_bassline_random,
    measure::{Measure, TimeSignature},
    music_xml::MusicXmlFormat,
    note::{Note, NoteEngraving, NoteLetter},
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
    use note::{NoteEngraving::*, NoteLetter::*, NotePlayed};

    let instruments = vec![
        Instrument::Violin, // Treble clef
        Instrument::BassGuitar,
    ];

    let key = Key::new(C, SharpFlat::Natural, MajorMinor::Minor);
    let sig = TimeSignature::new(6, 8);
    let tempo = 106;
    let divisions = 12;
    let amplitude = 0.2;

    let note = |letter, octave, engraving, voice, staff| NotePlayed {
        note: Note::new(letter, None, octave),
        engraving,
        duration: engraving.to_duration_ticks(divisions),
        amplitude,
        staff,
        voice,
    };

    let mut violin_measures = Vec::new();
    let mut bass_measures = Vec::new();

    let mut violin_m1 = Measure::new(key, sig, None, tempo);
    violin_m1.divisions = divisions;
    violin_m1.notes = vec![vec![
        note(C, 6, Eighth, 0, None),
        note(G, 5, Eighth, 0, None),
        note(F, 5, Half, 0, None),
    ]];
    violin_measures.push(violin_m1);

    let mut bass_m1 = Measure::new(key, sig, None, tempo);
    bass_m1.divisions = divisions;
    bass_m1.notes = vec![vec![
        note(C, 3, Eighth, 0, None),
        note(G, 3, Eighth, 0, None),
        note(C, 4, Eighth, 0, None),
        note(D, 4, Eighth, 0, None),
        note(E, 4, Quarter, 0, None),
    ]];
    bass_measures.push(bass_m1);

    let mut violin_m2 = Measure::new(key, sig, None, tempo);
    violin_m2.divisions = divisions;
    violin_m2.notes = vec![vec![
        note(C, 6, Eighth, 0, None),
        note(B, 5, Eighth, 0, None),
        note(E, 5, Half, 0, None),
    ]];
    violin_measures.push(violin_m2);

    let mut bass_m2 = Measure::new(key, sig, None, tempo);
    bass_m2.divisions = divisions;
    bass_m2.notes = vec![vec![
        note(C, 3, Eighth, 0, None),
        note(G, 3, Eighth, 0, None),
        note(D, 4, Eighth, 0, None),
        note(E, 4, Eighth, 0, None),
        note(F, 4, Quarter, 0, None),
    ]];
    bass_measures.push(bass_m2);

    Composition {
        metadata: Default::default(),
        measures_by_part: vec![
            (instruments[0], violin_measures),
            (instruments[1], bass_measures),
        ],
        temperament: Temperament::WellTempered(key),
    }
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

    let mut measures = vec![
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

    make_bassline_random(&mut measures, 0, true);

    Composition {
        metadata: Default::default(),
        measures_by_part: vec![(Instrument::BassGuitar, measures)],
        temperament: Temperament::WellTempered(key),
    }
}

/// Make a simple composition for piano using a chord progression. 4/4 time signature. The right hand of the piano,
/// on the treble clef, will play two half-note chords per measure. (The full chord). The left hand (bass clef)
/// will play `make_random_baseline()` of the chord.
///
/// One measure per chord.
fn make_comp_from_prog(key: Key, chords: &[Chord]) -> Composition {
    use crate::note::{NoteEngraving::Half, NotePlayed};

    let sig = TimeSignature::new(4, 4);
    let tempo = 100;
    let divisions = 8;
    let amplitude = 0.2;

    let mut measures: Vec<Measure> = chords
        .iter()
        .map(|chord| {
            let mut measure = Measure::new(key, sig, Some(chord.clone()), tempo);
            measure.divisions = divisions;
            measure
        })
        .collect();

    make_bassline_random(&mut measures, 0, true);
    for measure in &mut measures {
        for note in &mut measure.notes[0] {
            note.staff = Some(2);
        }
    }

    for (measure, chord) in measures.iter_mut().zip(chords) {
        let chord_notes = chord.notes();
        for (chord_voice_idx, chord_note) in chord_notes.into_iter().enumerate() {
            while measure.notes.len() <= chord_voice_idx + 1 {
                measure.notes.push(Vec::new());
            }

            let voice_idx = chord_voice_idx + 1;
            measure.notes[voice_idx].push(NotePlayed {
                note: chord_note.clone(),
                engraving: Half,
                duration: Half.to_duration_ticks(divisions),
                amplitude,
                staff: Some(1),
                voice: voice_idx,
            });
            measure.notes[voice_idx].push(NotePlayed {
                note: chord_note,
                engraving: Half,
                duration: Half.to_duration_ticks(divisions),
                amplitude,
                staff: Some(1),
                voice: voice_idx,
            });
        }
    }

    Composition {
        metadata: Default::default(),
        measures_by_part: vec![(Instrument::Piano, measures)],
        temperament: Temperament::WellTempered(key),
    }
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

    let comp_loaded = Composition::load_musicxml(Path::new(
        "C:/Users/the_a/compositions/training_etc/alicia-clair-obscur-expedition-33-main-theme-piano-solo.mxl",
    ))
    .unwrap_or_else(|_| make_test_composition());
    comp_loaded
        .save_midi(Path::new("./comp_loaded.mid"))
        .unwrap();
    comp_loaded
        .save_musicxml(MusicXmlFormat::Raw, Path::new("./comp_loaded.musicxml"))
        .unwrap();

    println!("\nComp: {comp_loaded}");

    for (instrument, measures) in &comp_loaded.measures_by_part {
        println!("Part: {:?}", std::mem::discriminant(instrument));
        println!("  measures: {}", measures.len());
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
