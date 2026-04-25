use std::path::Path;

use egui::{CentralPanel, Context, ThemePreference, Ui};

use crate::{State, composition::Composition, music_xml::MusicXmlFormat};

pub const WINDOW_WIDTH: f32 = 800.;
pub const WINDOW_HEIGHT: f32 = 600.;

pub(in crate::gui) const ROW_SPACING: f32 = 12.;
pub(in crate::gui) const COL_SPACING: f32 = 12.;

fn composition_list(comps: &[Composition], ui: &mut Ui) {
    for comp in comps {
        let title_sanitized_for_gui = comp.to_string().replace("♭", "b");
        ui.label(title_sanitized_for_gui);

        ui.horizontal(|ui| {
            if ui.button("Play").clicked() {
                comp.play();
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

        ui.separator();
        ui.add_space(ROW_SPACING);
    }
}

pub fn draw(state: &mut State, ctx: &Context) {
    // ctx.options_mut(|o| o.theme_preference = ThemePreference::Dark);

    CentralPanel::default().show(ctx, |ui| {
        ui.heading("Music");

        ui.add_space(ROW_SPACING);

        ui.label("Compositions");

        composition_list(&state.compositions, ui);
    });
}
