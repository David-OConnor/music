use std::path::Path;

use egui::{CentralPanel, ComboBox, Grid, Ui};

use crate::{
    ProgEditorChordUi, State,
    chord::{ChordDegree, ChordQuality, Inversion},
    composition::Composition,
    key_scale::{Key, MajorMinor, SharpFlat},
    music_xml::MusicXmlFormat,
    note::NoteLetter,
};

pub const WINDOW_WIDTH: f32 = 1200.;
pub const WINDOW_HEIGHT: f32 = 1000.;

pub(in crate::gui) const ROW_SPACING: f32 = 12.;
pub(in crate::gui) const COL_SPACING: f32 = 12.;

const MAJOR_KEY_TONICS: [(NoteLetter, SharpFlat); 15] = [
    (NoteLetter::C, SharpFlat::Natural),
    (NoteLetter::G, SharpFlat::Natural),
    (NoteLetter::D, SharpFlat::Natural),
    (NoteLetter::A, SharpFlat::Natural),
    (NoteLetter::E, SharpFlat::Natural),
    (NoteLetter::B, SharpFlat::Natural),
    (NoteLetter::F, SharpFlat::Sharp),
    (NoteLetter::C, SharpFlat::Sharp),
    (NoteLetter::F, SharpFlat::Natural),
    (NoteLetter::B, SharpFlat::Flat),
    (NoteLetter::E, SharpFlat::Flat),
    (NoteLetter::A, SharpFlat::Flat),
    (NoteLetter::D, SharpFlat::Flat),
    (NoteLetter::G, SharpFlat::Flat),
    (NoteLetter::C, SharpFlat::Flat),
];

const MINOR_KEY_TONICS: [(NoteLetter, SharpFlat); 15] = [
    (NoteLetter::A, SharpFlat::Natural),
    (NoteLetter::E, SharpFlat::Natural),
    (NoteLetter::B, SharpFlat::Natural),
    (NoteLetter::F, SharpFlat::Sharp),
    (NoteLetter::C, SharpFlat::Sharp),
    (NoteLetter::G, SharpFlat::Sharp),
    (NoteLetter::D, SharpFlat::Sharp),
    (NoteLetter::A, SharpFlat::Sharp),
    (NoteLetter::D, SharpFlat::Natural),
    (NoteLetter::G, SharpFlat::Natural),
    (NoteLetter::C, SharpFlat::Natural),
    (NoteLetter::F, SharpFlat::Natural),
    (NoteLetter::B, SharpFlat::Flat),
    (NoteLetter::E, SharpFlat::Flat),
    (NoteLetter::A, SharpFlat::Flat),
];

const TRIAD_INVERSIONS: [(Inversion, &str); 3] = [
    (Inversion::Root, "Root"),
    (Inversion::First, "1st"),
    (Inversion::Second, "2nd"),
];

const EXTENDED_INVERSIONS: [(Inversion, &str); 4] = [
    (Inversion::Root, "Root"),
    (Inversion::First, "1st"),
    (Inversion::Second, "2nd"),
    (Inversion::Third, "3rd"),
];

fn composition_list(comps: &[Composition], ui: &mut Ui) {
    for comp in comps {
        let title_sanitized_for_gui = comp.to_string().replace("♭", "b");
        ui.label(title_sanitized_for_gui);

        ui.horizontal(|ui| {
            if ui.button("Load").clicked() {
                // todo: File dialog using
            }

            if ui.button("Play").clicked() {
                if comp.play().is_err() {
                    eprintln!("Problem playing music")
                };
            }

            if ui.button("Save MusicXML").clicked() {
                let path = Path::new("./comp_loaded.musicxml");
                if comp.save_musicxml(MusicXmlFormat::Raw, path).is_err() {
                    eprintln!("Error saving musicxml");
                }
            }
            ui.add_space(COL_SPACING);

            if ui.button("Save MusicXML compressed").clicked() {
                let path = Path::new("./comp_loaded.mxl");
                if comp
                    .save_musicxml(MusicXmlFormat::Compressed, path)
                    .is_err()
                {
                    eprintln!("Error saving musicxml");
                }
            }
            ui.add_space(COL_SPACING);

            if ui.button("Save MIDI").clicked() {
                let path = Path::new("./comp_loaded.mid");
                if comp.save_midi(path).is_err() {
                    eprintln!("Error saving midi");
                }
            }
            ui.add_space(COL_SPACING);
        });

        let chords = {
            // todo: More checks for all chords present.
            let mut v = Vec::new();
            if !comp.measures_by_part.is_empty() {
                for meas in &comp.measures_by_part[0].1 {
                    if let Some(c) = &meas.chord {
                        v.push(c);
                    }
                }
            }
            v
        };

        if !chords.is_empty() {
            ui.label("Chords");
            ui.horizontal_wrapped(|ui| {
                for chord in chords {
                    ui.label(format!("- {chord}"));
                }
            });
        }

        ui.separator();
        ui.add_space(ROW_SPACING);
    }
}

/// Interface to generate chord progressions
fn chord_prog_maker(state: &mut State, ui: &mut Ui) {
    state
        .ui
        .prog_editor
        .sync_from_progression(&state.chord_prog_active);

    let mut dirty = false;
    let old_key = state.ui.prog_editor.key;
    normalize_key_choice(&mut state.ui.prog_editor.key);

    ui.group(|ui| {
        ui.heading("Chord Progression");

        ui.horizontal_wrapped(|ui| {
            ui.label("Key");
            let mut tonic = (
                state.ui.prog_editor.key.base_note,
                state.ui.prog_editor.key.sharp_flat,
            );

            ComboBox::from_id_salt("prog_editor_key_tonic")
                .selected_text(tonic_label(tonic.0, tonic.1))
                .show_ui(ui, |ui| {
                    for &(base_note, sharp_flat) in
                        supported_key_tonics(state.ui.prog_editor.key.major_minor)
                    {
                        ui.selectable_value(
                            &mut tonic,
                            (base_note, sharp_flat),
                            tonic_label(base_note, sharp_flat),
                        );
                    }
                });
            state.ui.prog_editor.key.base_note = tonic.0;
            state.ui.prog_editor.key.sharp_flat = tonic.1;

            ComboBox::from_id_salt("prog_editor_key_mode")
                .selected_text(state.ui.prog_editor.key.major_minor.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut state.ui.prog_editor.key.major_minor,
                        MajorMinor::Major,
                        MajorMinor::Major.to_string(),
                    );
                    ui.selectable_value(
                        &mut state.ui.prog_editor.key.major_minor,
                        MajorMinor::Minor,
                        MajorMinor::Minor.to_string(),
                    );
                });

            normalize_key_choice(&mut state.ui.prog_editor.key);
            if state.ui.prog_editor.key != old_key {
                let new_key = state.ui.prog_editor.key;
                sync_diatonic_quality_for_key_change(
                    &mut state.ui.prog_editor.chord_to_add,
                    old_key,
                    new_key,
                );
                for chord_ui in &mut state.ui.prog_editor.chords {
                    sync_diatonic_quality_for_key_change(chord_ui, old_key, new_key);
                }
                dirty = true;
            }
        });

        ui.add_space(ROW_SPACING / 2.0);

        ui.label("Add chord");
        ui.horizontal_wrapped(|ui| {
            ui.push_id("prog_editor_add_chord", |ui| {
                draw_chord_editor_controls(
                    ui,
                    state.ui.prog_editor.key,
                    &mut state.ui.prog_editor.chord_to_add,
                );
            });
            ui.monospace(
                state
                    .ui
                    .prog_editor
                    .chord_to_add
                    .to_chord(state.ui.prog_editor.key)
                    .to_string(),
            );
            if ui.button("Add").clicked() {
                state
                    .ui
                    .prog_editor
                    .chords
                    .push(state.ui.prog_editor.chord_to_add.clone());
                dirty = true;
            }
        });

        ui.add_space(ROW_SPACING / 2.0);
        ui.separator();
        ui.add_space(ROW_SPACING / 2.0);

        if state.ui.prog_editor.chords.is_empty() {
            ui.label("No chords in the active progression yet.");
        } else {
            let key = state.ui.prog_editor.key;
            let mut remove_idx = None;

            Grid::new("prog_editor_chords_grid")
                .num_columns(7)
                .striped(true)
                .spacing([COL_SPACING, ROW_SPACING / 2.0])
                .show(ui, |ui| {
                    ui.strong("#");
                    ui.strong("Degree");
                    ui.strong("Quality");
                    ui.strong("Extension");
                    ui.strong("Inversion");
                    ui.strong("Preview");
                    ui.strong("X");
                    ui.end_row();

                    for (idx, chord_ui) in state.ui.prog_editor.chords.iter_mut().enumerate() {
                        let before = chord_ui.clone();

                        ui.label((idx + 1).to_string());
                        ui.push_id(idx, |ui| {
                            draw_chord_editor_controls(ui, key, chord_ui);
                        });
                        ui.monospace(chord_ui.to_chord(key).to_string());
                        if ui.small_button("X").clicked() {
                            remove_idx = Some(idx);
                        }
                        ui.end_row();

                        if *chord_ui != before {
                            dirty = true;
                        }
                    }
                });

            if let Some(remove_idx) = remove_idx {
                state.ui.prog_editor.chords.remove(remove_idx);
                dirty = true;
            }
        }
    });

    if dirty {
        state.chord_prog_active = state.ui.prog_editor.rebuild_progression();
    }
}

fn tonic_label(base_note: NoteLetter, sharp_flat: SharpFlat) -> String {
    let accidental = match sharp_flat {
        SharpFlat::Natural => "",
        SharpFlat::Sharp => "#",
        SharpFlat::Flat => "b",
    };

    format!("{base_note}{accidental}")
}

fn supported_key_tonics(mode: MajorMinor) -> &'static [(NoteLetter, SharpFlat)] {
    match mode {
        MajorMinor::Major => &MAJOR_KEY_TONICS,
        MajorMinor::Minor => &MINOR_KEY_TONICS,
    }
}

fn normalize_key_choice(key: &mut Key) {
    let supported = supported_key_tonics(key.major_minor);

    if supported
        .iter()
        .any(|&(base_note, sharp_flat)| base_note == key.base_note && sharp_flat == key.sharp_flat)
    {
        return;
    }

    if supported
        .iter()
        .any(|&(base_note, sharp_flat)| base_note == key.base_note && sharp_flat == SharpFlat::Natural)
    {
        key.sharp_flat = SharpFlat::Natural;
        return;
    }

    let (base_note, sharp_flat) = supported[0];
    key.base_note = base_note;
    key.sharp_flat = sharp_flat;
}

fn quality_label(quality: ChordQuality) -> &'static str {
    match quality {
        ChordQuality::Major => "Major",
        ChordQuality::Minor => "Minor",
        ChordQuality::Dominant => "Dominant",
        ChordQuality::Augmented => "Augmented",
        ChordQuality::Diminished => "Diminished",
    }
}

fn extension_label(extension: Option<u8>) -> &'static str {
    match extension {
        None => "Triad",
        Some(7) => "7",
        Some(9) => "9",
        Some(11) => "11",
        Some(13) => "13",
        Some(_) => "Other",
    }
}

fn inversion_label(inversion: Inversion) -> &'static str {
    match inversion {
        Inversion::Root => "Root",
        Inversion::First => "1st",
        Inversion::Second => "2nd",
        Inversion::Third => "3rd",
    }
}

fn supported_inversions(extension: Option<u8>) -> &'static [(Inversion, &'static str)] {
    if extension.is_some() {
        &EXTENDED_INVERSIONS
    } else {
        &TRIAD_INVERSIONS
    }
}

fn normalize_editor_chord(chord_ui: &mut ProgEditorChordUi) {
    if chord_ui.quality == ChordQuality::Dominant && chord_ui.extension.is_none() {
        chord_ui.extension = Some(7);
    }

    if chord_ui.extension.is_none() && chord_ui.inversion == Inversion::Third {
        chord_ui.inversion = Inversion::Second;
    }
}

fn sync_diatonic_quality_for_key_change(
    chord_ui: &mut ProgEditorChordUi,
    old_key: Key,
    new_key: Key,
) {
    if chord_ui.extension.is_none()
        && chord_ui.alterations.is_empty()
        && chord_ui.quality == old_key.diatonic_quality(chord_ui.degree)
    {
        chord_ui.quality = new_key.diatonic_quality(chord_ui.degree);
    }

    normalize_editor_chord(chord_ui);
}

fn draw_chord_editor_controls(ui: &mut Ui, key: Key, chord_ui: &mut ProgEditorChordUi) {
    let prev_degree = chord_ui.degree;
    let prev_quality = chord_ui.quality;
    let prev_extension = chord_ui.extension;

    ComboBox::from_id_salt("degree")
        .selected_text(chord_ui.degree.to_string(key, Inversion::Root))
        .show_ui(ui, |ui| {
            for degree in ChordDegree::all() {
                ui.selectable_value(
                    &mut chord_ui.degree,
                    degree,
                    degree.to_string(key, Inversion::Root),
                );
            }
        });

    if chord_ui.degree != prev_degree
        && chord_ui.extension.is_none()
        && chord_ui.alterations.is_empty()
        && prev_quality == key.diatonic_quality(prev_degree)
    {
        chord_ui.quality = key.diatonic_quality(chord_ui.degree);
    }

    ComboBox::from_id_salt("quality")
        .selected_text(quality_label(chord_ui.quality))
        .show_ui(ui, |ui| {
            for quality in [
                ChordQuality::Major,
                ChordQuality::Minor,
                ChordQuality::Dominant,
                ChordQuality::Augmented,
                ChordQuality::Diminished,
            ] {
                ui.selectable_value(&mut chord_ui.quality, quality, quality_label(quality));
            }
        });

    ComboBox::from_id_salt("extension")
        .selected_text(extension_label(chord_ui.extension))
        .show_ui(ui, |ui| {
            for extension in [None, Some(7), Some(9), Some(11), Some(13)] {
                ui.selectable_value(
                    &mut chord_ui.extension,
                    extension,
                    extension_label(extension),
                );
            }
        });

    if prev_extension != chord_ui.extension
        && chord_ui.extension.is_none()
        && chord_ui.quality == ChordQuality::Dominant
    {
        chord_ui.quality = ChordQuality::Major;
    }

    normalize_editor_chord(chord_ui);

    ComboBox::from_id_salt("inversion")
        .selected_text(inversion_label(chord_ui.inversion))
        .show_ui(ui, |ui| {
            for &(inversion, label) in supported_inversions(chord_ui.extension) {
                ui.selectable_value(&mut chord_ui.inversion, inversion, label);
            }
        });

    normalize_editor_chord(chord_ui);
}

pub fn draw(state: &mut State, ui: &mut Ui) {
    // ctx.options_mut(|o| o.theme_preference = ThemePreference::Dark);

    CentralPanel::default().show_inside(ui, |ui| {
        ui.heading("Music");

        ui.add_space(ROW_SPACING);

        ui.label("Compositions");

        composition_list(&state.compositions, ui);

        ui.add_space(ROW_SPACING);

        chord_prog_maker(state, ui);
    });
}
