//! The "Add ..." buttons in the sidebar: a wide button with a tiny line
//! drawing of the piece on the left.

use bevy_egui::egui;

use crate::layout::model::{CornerStyle, Kind};

/// A full-width button with a small icon of `kind` and a label.
pub(super) fn library_button(ui: &mut egui::Ui, label: &str, kind: Kind) -> egui::Response {
    let desired = egui::vec2(ui.available_width(), 36.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let visuals = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        visuals.corner_radius,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let icon = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 7.0), egui::vec2(24.0, 22.0));
    paint_kind_icon(ui.painter(), icon, kind, visuals.fg_stroke);
    ui.painter().text(
        egui::pos2(rect.left() + 42.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        visuals.fg_stroke.color,
    );
    response
}

/// A tiny line drawing of each kind: drawer lines, a door split, a sink
/// bowl, and so on. Drawn with egui shapes, so there are no image files.
fn paint_kind_icon(painter: &egui::Painter, rect: egui::Rect, kind: Kind, stroke: egui::Stroke) {
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
    let line = |a: egui::Pos2, b: egui::Pos2| painter.line_segment([a, b], stroke);
    match kind {
        Kind::Drawers { .. } => {
            for fraction in [0.33, 0.66] {
                line(
                    egui::pos2(rect.left(), egui::lerp(rect.y_range(), fraction)),
                    egui::pos2(rect.right(), egui::lerp(rect.y_range(), fraction)),
                );
            }
        }
        Kind::JollyDrawers { .. } => {
            let split = egui::lerp(rect.x_range(), 0.35);
            line(
                egui::pos2(split, rect.top()),
                egui::pos2(split, rect.bottom()),
            );
            let middle = rect.center().y;
            line(egui::pos2(split, middle), egui::pos2(rect.right(), middle));
        }
        Kind::HingedDoors { .. } | Kind::SlidingDoors => {
            line(
                egui::pos2(rect.center().x, rect.top()),
                egui::pos2(rect.center().x, rect.bottom()),
            );
        }
        Kind::LiftDoor => {
            line(rect.left_bottom(), rect.right_top());
        }
        Kind::Table => {
            line(
                egui::pos2(rect.left() + 4.0, rect.top() + 6.0),
                egui::pos2(rect.left() + 4.0, rect.bottom()),
            );
            line(
                egui::pos2(rect.right() - 4.0, rect.top() + 6.0),
                egui::pos2(rect.right() - 4.0, rect.bottom()),
            );
        }
        Kind::Countertop | Kind::Tabletop => {
            line(
                egui::pos2(rect.left(), rect.top() + 6.0),
                egui::pos2(rect.right(), rect.top() + 6.0),
            );
        }
        Kind::Pipe { horizontal, .. } => {
            if horizontal {
                line(rect.left_center(), rect.right_center());
            } else {
                line(rect.center_top(), rect.center_bottom());
            }
        }
        Kind::ElectricOven | Kind::Microwave | Kind::CoffeeMachine => {
            painter.rect_stroke(
                rect.shrink(5.0),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Kind::InductionHob => {
            let radius = rect.width().min(rect.height()) * 0.14;
            for (x, y) in [(0.3, 0.32), (0.7, 0.32), (0.3, 0.68), (0.7, 0.68)] {
                painter.circle_stroke(
                    egui::pos2(egui::lerp(rect.x_range(), x), egui::lerp(rect.y_range(), y)),
                    radius,
                    stroke,
                );
            }
        }
        Kind::SquareSink => {
            painter.rect_stroke(
                rect.shrink(5.0),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Kind::RoundSink => {
            painter.circle_stroke(
                rect.center(),
                rect.width().min(rect.height()) * 0.32,
                stroke,
            );
        }
        Kind::DoubleSink => {
            let left = egui::Rect::from_min_max(
                rect.min + egui::vec2(3.0, 5.0),
                egui::pos2(rect.center().x - 1.0, rect.bottom() - 5.0),
            );
            let right = egui::Rect::from_min_max(
                egui::pos2(rect.center().x + 1.0, rect.top() + 5.0),
                rect.max - egui::vec2(3.0, 5.0),
            );
            painter.rect_stroke(left, 1.0, stroke, egui::StrokeKind::Inside);
            painter.rect_stroke(right, 1.0, stroke, egui::StrokeKind::Inside);
        }
        Kind::SinkFaucet => {
            line(rect.center_bottom(), rect.center_top());
            line(
                rect.center_top(),
                egui::pos2(rect.right() - 3.0, rect.top()),
            );
            line(
                egui::pos2(rect.right() - 3.0, rect.top()),
                egui::pos2(rect.right() - 3.0, rect.center().y),
            );
        }
        Kind::CentralHeatingBoiler => {
            painter.rect_stroke(
                rect.shrink(4.0),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
            line(
                egui::pos2(rect.left() + 6.0, rect.bottom() - 5.0),
                egui::pos2(rect.right() - 6.0, rect.bottom() - 5.0),
            );
        }
        Kind::AirConditioner => {
            line(
                egui::pos2(rect.left() + 3.0, rect.bottom() - 6.0),
                egui::pos2(rect.right() - 3.0, rect.bottom() - 6.0),
            );
            line(
                egui::pos2(rect.left() + 5.0, rect.bottom() - 3.0),
                egui::pos2(rect.right() - 5.0, rect.bottom() - 3.0),
            );
        }
        Kind::Refrigerator => {
            line(
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            );
        }
        Kind::Dishwasher => {
            line(
                egui::pos2(rect.left() + 3.0, rect.top() + 5.0),
                egui::pos2(rect.right() - 3.0, rect.top() + 5.0),
            );
        }
        Kind::OpenShelves { shelves } => {
            for index in 1..=shelves {
                let fraction = index as f32 / (shelves as f32 + 1.0);
                line(
                    egui::pos2(rect.left(), egui::lerp(rect.y_range(), fraction)),
                    egui::pos2(rect.right(), egui::lerp(rect.y_range(), fraction)),
                );
            }
        }
        Kind::Corner { style } => {
            line(rect.left_top(), rect.left_bottom());
            line(rect.left_bottom(), rect.right_bottom());
            line(
                egui::pos2(rect.left(), rect.top() + rect.height() * 0.42),
                egui::pos2(
                    rect.left() + rect.width() * 0.42,
                    rect.top() + rect.height() * 0.42,
                ),
            );
            line(
                egui::pos2(
                    rect.left() + rect.width() * 0.42,
                    rect.top() + rect.height() * 0.42,
                ),
                egui::pos2(rect.left() + rect.width() * 0.42, rect.bottom()),
            );
            if matches!(style, CornerStyle::DoubleDoor) {
                line(rect.right_top(), rect.left_bottom());
            }
        }
        Kind::Custom => {
            line(
                egui::pos2(rect.left() + 4.0, rect.bottom() - 4.0),
                egui::pos2(rect.right() - 4.0, rect.bottom() - 4.0),
            );
            line(
                egui::pos2(rect.left() + 4.0, rect.top() + 4.0),
                egui::pos2(rect.left() + 4.0, rect.bottom() - 4.0),
            );
            line(
                egui::pos2(rect.left() + 8.0, rect.center().y),
                egui::pos2(rect.right() - 6.0, rect.center().y),
            );
        }
    }
}
