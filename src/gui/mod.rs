use egui::{CentralPanel, Context, ThemePreference};

use crate::State;

pub const WINDOW_WIDTH: f32 = 800.;
pub const WINDOW_HEIGHT: f32 = 600.;

pub(in crate::gui) const ROW_SPACING: f32 = 12.;
pub(in crate::gui) const COL_SPACING: f32 = 12.;

pub fn draw(state: &mut State, ctx: &Context) {
    // ctx.options_mut(|o| o.theme_preference = ThemePreference::Dark);

    CentralPanel::default().show(ctx, |ui| {
        ui.heading("Music");

        ui.add_space(ROW_SPACING);

        ui.label("Compositions");
        for comp in &state.compositions {
            let title_sanitized_for_gui = comp.to_string().replace("♭", "b");
            ui.label(title_sanitized_for_gui);

            ui.add_space(ROW_SPACING);
        }
    });
}
