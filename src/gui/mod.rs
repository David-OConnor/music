use std::path::Path;

use egui::{
    Align, Button, CentralPanel, Color32, ComboBox, DragValue, Layout, ScrollArea, TextEdit, Ui,
    vec2,
};

use crate::{
    CompositionGuideMeasureUi, GuideEditorFeedback, ProgEditorChordUi, RhythmPatternUi, State,
    chord::{ChordDegree, ChordQuality, Inversion},
    composition::Composition,
    key_scale::{Key, MajorMinor, SharpFlat},
    measure::TimeSignature,
    music_xml::MusicXmlFormat,
    note::NoteLetter,
    player,
    rhythm::RhythmPattern,
};

pub const WINDOW_WIDTH: f32 = 1200.;
pub const WINDOW_HEIGHT: f32 = 1000.;

pub(in crate::gui) const ROW_SPACING: f32 = 12.;
pub(in crate::gui) const COL_SPACING: f32 = 12.;

/// Fixed width for each measure card in the guide editor's wrapping row.
/// Bounding the width is what lets `horizontal_wrapped` actually wrap the cards.
const MEASURE_CARD_WIDTH: f32 = 380.;

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

fn composition_list(state: &mut State, ui: &mut Ui) -> bool {
    if let Some((_, playback)) = &state.current_playback {
        if playback.is_finished() {
            state.current_playback = None;
        }
    }

    let mut load_clicked = false;
    for (idx, comp) in state.compositions.iter().enumerate() {
        let title_sanitized_for_gui = comp.to_string().replace("♭", "b");
        ui.label(title_sanitized_for_gui);

        ui.horizontal(|ui| {
            if ui.button("Load").clicked() {
                load_clicked = true;
            }

            if ui.button("Play").clicked() {
                if let Some((_, prev)) = &state.current_playback {
                    player::stop_playing(prev);
                }
                state.current_playback = None;

                match comp.play() {
                    Ok(Some(playback)) => {
                        state.current_playback = Some((idx, playback));
                    }
                    Ok(None) => {}
                    Err(_) => eprintln!("Problem playing music"),
                }
            }

            let is_playing = matches!(
                &state.current_playback,
                Some((playing_idx, _)) if *playing_idx == idx,
            );
            if is_playing && ui.button("Stop").clicked() {
                if let Some((_, playback)) = &state.current_playback {
                    player::stop_playing(playback);
                }
                state.current_playback = None;
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
    load_clicked
}

fn composition_guide_editor(state: &mut State, ui: &mut Ui) {
    let mut dirty = false;
    let mut wrote_feedback = false;
    let old_key = state.ui.guide_editor.key;
    normalize_key_choice(&mut state.ui.guide_editor.key);

    ui.group(|ui| {
        ui.heading("Composition Guide");

        ui.horizontal_wrapped(|ui| {
            ui.label("Key:");
            let mut tonic = (
                state.ui.guide_editor.key.base_note,
                state.ui.guide_editor.key.sharp_flat,
            );

            ComboBox::from_id_salt("guide_editor_key_tonic")
                .width(60.)
                .selected_text(tonic_label(tonic.0, tonic.1))
                .show_ui(ui, |ui| {
                    for &(base_note, sharp_flat) in
                        supported_key_tonics(state.ui.guide_editor.key.major_minor)
                    {
                        ui.selectable_value(
                            &mut tonic,
                            (base_note, sharp_flat),
                            tonic_label(base_note, sharp_flat),
                        );
                    }
                });
            state.ui.guide_editor.key.base_note = tonic.0;
            state.ui.guide_editor.key.sharp_flat = tonic.1;

            ComboBox::from_id_salt("guide_editor_key_mode")
                .selected_text(state.ui.guide_editor.key.major_minor.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut state.ui.guide_editor.key.major_minor,
                        MajorMinor::Major,
                        MajorMinor::Major.to_string(),
                    );
                    ui.selectable_value(
                        &mut state.ui.guide_editor.key.major_minor,
                        MajorMinor::Minor,
                        MajorMinor::Minor.to_string(),
                    );
                });

            ui.add_space(COL_SPACING);
            ui.label("Time signature:");
            ui.add(DragValue::new(&mut state.ui.guide_editor.time_sig_numerator).range(1..=32));
            ui.label("/");
            ComboBox::from_id_salt("guide_editor_time_sig_denominator")
                .width(48.)
                .selected_text(state.ui.guide_editor.time_sig_denominator.to_string())
                .show_ui(ui, |ui| {
                    for denominator in [1_u8, 2, 4, 8, 16, 32] {
                        ui.selectable_value(
                            &mut state.ui.guide_editor.time_sig_denominator,
                            denominator,
                            denominator.to_string(),
                        );
                    }
                });

            ui.add_space(COL_SPACING);
            ui.label("Tempo");
            ui.add(DragValue::new(&mut state.ui.guide_editor.tempo).range(20..=300));
            ui.label("BPM");

            normalize_key_choice(&mut state.ui.guide_editor.key);
            if state.ui.guide_editor.key != old_key {
                let new_key = state.ui.guide_editor.key;
                sync_add_chord_quality(&mut state.ui.guide_editor.measure_to_add.chord, new_key);
                for measure_ui in &mut state.ui.guide_editor.measures {
                    sync_diatonic_quality_for_key_change(&mut measure_ui.chord, old_key, new_key);
                }
                dirty = true;
            }
        });

        ui.add_space(ROW_SPACING / 2.0);
        ui.label("Add measure");

        let add_key = state.ui.guide_editor.key;
        let add_sig = state.ui.guide_editor.time_signature();
        ui.push_id("guide_editor_add_measure", |ui| {
            draw_measure_editor(
                ui,
                add_sig,
                add_key,
                &mut state.ui.guide_editor.measure_to_add,
                true,
            );
        });

        ui.horizontal_wrapped(|ui| {
            ui.monospace(measure_preview(
                add_key,
                &state.ui.guide_editor.measure_to_add,
            ));
            if ui.button("Add measure").clicked() {
                let measure_to_add = state.ui.guide_editor.measure_to_add.clone();
                state.ui.guide_editor.measures.push(measure_to_add);
                dirty = true;
            }
        });

        ui.add_space(ROW_SPACING / 2.0);
        ui.separator();
        ui.add_space(ROW_SPACING / 2.0);

        if state.ui.guide_editor.measures.is_empty() {
            ui.label("No measures in the active composition guide yet.");
        } else {
            let key = state.ui.guide_editor.key;
            let sig = state.ui.guide_editor.time_signature();
            let mut remove_idx = None;

            let cards_per_row =
                ((ui.available_width() / (MEASURE_CARD_WIDTH + COL_SPACING)).floor() as usize)
                    .max(1);

            let measures = &mut state.ui.guide_editor.measures;
            let total = measures.len();
            let mut row_start = 0;
            while row_start < total {
                let row_end = (row_start + cards_per_row).min(total);
                ui.horizontal_top(|ui| {
                    for idx in row_start..row_end {
                        let measure_ui = &mut measures[idx];
                        let before = measure_ui.clone();

                        ui.allocate_ui_with_layout(
                            vec2(MEASURE_CARD_WIDTH, 0.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.set_max_width(MEASURE_CARD_WIDTH);
                                ui.group(|ui| {
                                    ui.set_max_width(MEASURE_CARD_WIDTH);
                                    ui.horizontal(|ui| {
                                        ui.strong(format!("Measure {}", idx + 1));
                                        if ui.small_button("Remove").clicked() {
                                            remove_idx = Some(idx);
                                        }
                                    });
                                    ui.monospace(measure_preview(key, measure_ui));
                                    ui.add_space(ROW_SPACING / 3.0);
                                    ui.push_id(idx, |ui| {
                                        draw_measure_editor(ui, sig, key, measure_ui, false);
                                    });
                                });
                            },
                        );

                        if *measure_ui != before {
                            dirty = true;
                        }
                    }
                });
                ui.add_space(ROW_SPACING / 2.0);
                row_start = row_end;
            }

            if let Some(remove_idx) = remove_idx {
                state.ui.guide_editor.measures.remove(remove_idx);
                dirty = true;
            }
        }

        if ui
            .add_enabled(
                !state.ui.guide_editor.measures.is_empty(),
                Button::new("Make composition"),
            )
            .clicked()
        {
            match state.ui.guide_editor.build_guide() {
                Ok(guide) => {
                    let measure_count = guide.chords.len();
                    match guide.make_comp() {
                        Ok(comp) => {
                            state.compositions.push(comp);
                            state.ui.guide_editor.feedback = Some(GuideEditorFeedback {
                                text: format!("Added composition with {measure_count} measure(s)."),
                                is_error: false,
                            });
                            wrote_feedback = true;
                        }
                        Err(err) => {
                            state.ui.guide_editor.feedback = Some(GuideEditorFeedback {
                                text: format!("Could not make composition: {err}"),
                                is_error: true,
                            });
                            wrote_feedback = true;
                        }
                    }
                }
                Err(err) => {
                    state.ui.guide_editor.feedback = Some(GuideEditorFeedback {
                        text: err,
                        is_error: true,
                    });
                    wrote_feedback = true;
                }
            }
        }

        if let Some(feedback) = &state.ui.guide_editor.feedback {
            let color = if feedback.is_error {
                Color32::from_rgb(180, 40, 40)
            } else {
                Color32::from_rgb(40, 120, 40)
            };
            ui.add_space(ROW_SPACING / 2.0);
            ui.colored_label(color, &feedback.text);
        }
    });

    if dirty && !wrote_feedback {
        state.ui.guide_editor.feedback = None;
    }
}

fn draw_measure_editor(
    ui: &mut Ui,
    time_sig: TimeSignature,
    key: Key,
    measure_ui: &mut CompositionGuideMeasureUi,
    auto_sync_quality_for_degree: bool,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Chord:");
        draw_chord_editor_controls(ui, key, &mut measure_ui.chord, auto_sync_quality_for_degree);
    });
    ui.add_space(ROW_SPACING / 3.0);

    draw_rhythm_pattern_controls(ui, time_sig, &mut measure_ui.rhythm);
}

fn draw_rhythm_pattern_controls(
    ui: &mut Ui,
    time_sig: TimeSignature,
    rhythm_ui: &mut RhythmPatternUi,
) {
    ui.label("Rhythm pattern");
    ui.horizontal_wrapped(|ui| {
        if ui.small_button("Downbeats").clicked() {
            *rhythm_ui = RhythmPatternUi::from_pattern(&RhythmPattern::measure_downbeats(time_sig));
        }
        if ui.small_button("Syncopated").clicked() {
            *rhythm_ui = RhythmPatternUi::from_pattern(&RhythmPattern::syncopated(time_sig));
        }
        ui.monospace(rhythm_summary(rhythm_ui));
    });

    draw_hit_spec_editor(
        ui,
        "Primary",
        &mut rhythm_ui.primary_division,
        &mut rhythm_ui.primary_hits,
    );
    draw_hit_spec_editor(
        ui,
        "Secondary",
        &mut rhythm_ui.secondary_division,
        &mut rhythm_ui.secondary_hits,
    );
    draw_hit_spec_editor(
        ui,
        "Tertiary",
        &mut rhythm_ui.tertiary_division,
        &mut rhythm_ui.tertiary_hits,
    );

    if let Err(err) = rhythm_ui.to_pattern() {
        ui.colored_label(Color32::from_rgb(180, 40, 40), err);
    }
}

fn draw_hit_spec_editor(ui: &mut Ui, label: &str, division: &mut u8, hits: &mut String) {
    ui.horizontal_wrapped(|ui| {
        ui.label(label);
        ui.label("division");
        ui.add(DragValue::new(division).range(0..=32));
        ui.label("hits");
        ui.add(
            TextEdit::singleline(hits)
                .desired_width(150.0)
                .hint_text("0,2,3"),
        );
    });
}

fn measure_preview(key: Key, measure_ui: &CompositionGuideMeasureUi) -> String {
    format!(
        "{} | {}",
        measure_ui.chord.to_chord(key),
        rhythm_summary(&measure_ui.rhythm)
    )
}

fn rhythm_summary(rhythm_ui: &RhythmPatternUi) -> String {
    format!(
        "P {} | S {} | T {}",
        raw_hit_spec_summary(rhythm_ui.primary_division, &rhythm_ui.primary_hits),
        raw_hit_spec_summary(rhythm_ui.secondary_division, &rhythm_ui.secondary_hits),
        raw_hit_spec_summary(rhythm_ui.tertiary_division, &rhythm_ui.tertiary_hits),
    )
}

fn raw_hit_spec_summary(division: u8, hits: &str) -> String {
    if division == 0 {
        return "-".to_string();
    }

    let cleaned = hits
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(",");

    if cleaned.is_empty() {
        format!("{division}:[]")
    } else {
        format!("{division}:[{cleaned}]")
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

    if supported.iter().any(|&(base_note, sharp_flat)| {
        base_note == key.base_note && sharp_flat == SharpFlat::Natural
    }) {
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

fn sync_add_chord_quality(chord_ui: &mut ProgEditorChordUi, key: Key) {
    chord_ui.quality = key.diatonic_quality(chord_ui.degree);
    normalize_editor_chord(chord_ui);
}

fn draw_chord_editor_controls(
    ui: &mut Ui,
    key: Key,
    chord_ui: &mut ProgEditorChordUi,
    auto_sync_quality_for_degree: bool,
) {
    let prev_degree = chord_ui.degree;
    let prev_quality = chord_ui.quality;
    let prev_extension = chord_ui.extension;

    ComboBox::from_id_salt("degree")
        .width(48.)
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

    if chord_ui.degree != prev_degree {
        if auto_sync_quality_for_degree {
            sync_add_chord_quality(chord_ui, key);
        } else if chord_ui.extension.is_none()
            && chord_ui.alterations.is_empty()
            && prev_quality == key.diatonic_quality(prev_degree)
        {
            chord_ui.quality = key.diatonic_quality(chord_ui.degree);
        }
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
        .width(48.)
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
        .width(48.)
        .selected_text(inversion_label(chord_ui.inversion))
        .show_ui(ui, |ui| {
            for &(inversion, label) in supported_inversions(chord_ui.extension) {
                ui.selectable_value(&mut chord_ui.inversion, inversion, label);
            }
        });

    normalize_editor_chord(chord_ui);
}

pub fn draw(state: &mut State, ui: &mut Ui) {
    state.ui.file_dialog.update(ui.ctx());

    if let Some(path) = state.ui.file_dialog.take_picked() {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "mid" | "midi" => match Composition::load_midi(&path) {
                Ok(comp) => state.compositions.push(comp),
                Err(e) => eprintln!("Error loading MIDI: {e}"),
            },
            "mxl" | "musicxml" => match Composition::load_musicxml(&path) {
                Ok(comp) => state.compositions.push(comp),
                Err(e) => eprintln!("Error loading MusicXML: {e}"),
            },
            _ => eprintln!("Unsupported file format: .{ext}"),
        }
    }

    CentralPanel::default().show_inside(ui, |ui| {
        ScrollArea::vertical()
            // .min_scrolled_height(400.0)
            .show(ui, |ui| {
                ui.label("Compositions");

                if composition_list(state, ui) {
                    state.ui.file_dialog.pick_file();
                }

                ui.add_space(ROW_SPACING);

                composition_guide_editor(state, ui);
            });
    });
}
