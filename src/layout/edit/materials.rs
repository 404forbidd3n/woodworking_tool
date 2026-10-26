//! Color pickers used by the Properties panel.
//!
//! Each picker is a row of paint swatches and a row of wood swatches. Every
//! function returns `true` when the user changed something, so the caller
//! can mark the piece dirty and rebuild its mesh.

use bevy_egui::egui;

use crate::layout::model::{Finish, FinishPreset};
use crate::theme;

/// One picker for several parts at once (all carcass panels, or all fronts).
/// If they differ, the title says "mixed" and no swatch is highlighted.
/// Picking a swatch sets all of them.
pub(super) fn finish_group(ui: &mut egui::Ui, label: &str, parts: &mut [Finish]) -> bool {
    if parts.is_empty() {
        return false;
    }
    let uniform = parts.iter().all(|part| *part == parts[0]);
    let title = if uniform {
        label.to_owned()
    } else {
        format!("{label} · mixed")
    };
    let mut sample = parts[0];
    let changed = finish_picker(ui, &title, &mut sample, uniform);
    if changed {
        parts.fill(sample);
    }
    changed
}

/// One picker for one part.
pub(super) fn finish_row(ui: &mut egui::Ui, label: &str, finish: &mut Finish) -> bool {
    finish_picker(ui, label, finish, true)
}

/// The label, a row of paints, a row of woods, and a color picker when the
/// finish is custom.
fn finish_picker(ui: &mut egui::Ui, label: &str, finish: &mut Finish, highlight: bool) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new(label).color(theme::muted()));
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new("Paint")
                .size(12.0)
                .color(theme::heading()),
        );
        for preset in FinishPreset::PAINTS {
            changed |= finish_swatch(ui, finish, preset, highlight);
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new("Wood")
                .size(12.0)
                .color(theme::heading()),
        );
        for preset in FinishPreset::WOODS {
            changed |= finish_swatch(ui, finish, preset, highlight);
        }
    });
    if highlight && finish.preset == FinishPreset::Custom {
        ui.horizontal(|ui| {
            ui.label("Custom");
            changed |= ui.color_edit_button_srgb(&mut finish.color).changed();
        });
    }
    changed
}

/// One swatch button, filled with the finish's color.
fn finish_swatch(
    ui: &mut egui::Ui,
    finish: &mut Finish,
    preset: FinishPreset,
    highlight: bool,
) -> bool {
    let rgb = preset.rgb();
    let fill = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    let bright = rgb[0] as u16 + rgb[1] as u16 + rgb[2] as u16;
    let text = if bright > 430 {
        egui::Color32::from_rgb(28, 28, 30)
    } else {
        egui::Color32::WHITE
    };
    let selected = highlight && finish.preset == preset;
    let response = ui.add(
        egui::Button::new(egui::RichText::new(preset.label()).size(12.0).color(text))
            .fill(fill)
            .stroke(egui::Stroke::new(
                if selected { 2.0 } else { 1.0 },
                if selected {
                    theme::heading()
                } else {
                    egui::Color32::from_rgb(20, 20, 22)
                },
            ))
            .corner_radius(egui::CornerRadius::same(4)),
    );
    if response.clicked() && !selected {
        finish.preset = preset;
        finish.color = rgb;
        true
    } else {
        false
    }
}

/// Appliances only offer three body colors.
pub(super) fn appliance_finish_row(ui: &mut egui::Ui, finish: &mut Finish) -> bool {
    let choices = [
        (
            "Silver metal",
            Finish::new(FinishPreset::StainlessSteel, [165, 172, 175]),
        ),
        ("Black", Finish::new(FinishPreset::Custom, [25, 27, 29])),
        ("Dark gray", Finish::new(FinishPreset::Custom, [70, 73, 76])),
    ];
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label("Appliance color");
        for (label, option) in choices {
            if ui.selectable_label(*finish == option, label).clicked() && *finish != option {
                *finish = option;
                changed = true;
            }
        }
    });
    changed
}
