//! Stylized appliances: refrigerator, dishwasher, oven, coffee machine,
//! microwave, faucet, boiler, and air conditioner. Sinks have their own file.

use bevy::prelude::*;

use super::sink::{rectangular_sink, round_sink};
use super::{Solid, Surface};
use crate::layout::model::kind::Kind;

/// Recognizable but deliberately lightweight appliance models. They are made
/// from boxes so projects stay self-contained and do not depend on asset files.
pub(super) fn appliance(size: Vec3, kind: Kind) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let body = Solid::axis(size, Vec3::new(0.0, h * 0.5, 0.0), Surface::Appliance(0));
    let front_z = d * 0.5 + 0.006;
    match kind {
        Kind::Refrigerator => vec![
            body,
            Solid::axis(
                Vec3::new((w - 0.02).max(0.02), (h * 0.58 - 0.01).max(0.02), 0.012),
                Vec3::new(0.0, h * 0.71, front_z),
                Surface::Appliance(1),
            ),
            Solid::axis(
                Vec3::new((w - 0.02).max(0.02), (h * 0.42 - 0.01).max(0.02), 0.012),
                Vec3::new(0.0, h * 0.21, front_z),
                Surface::Appliance(1),
            ),
            appliance_handle(Vec3::new(w * 0.36, h * 0.7, front_z + 0.015), h * 0.34),
        ],
        Kind::Dishwasher => vec![
            body,
            Solid::axis(
                Vec3::new((w - 0.02).max(0.02), (h - 0.04).max(0.02), 0.014),
                Vec3::new(0.0, h * 0.5, front_z),
                Surface::Appliance(1),
            ),
            appliance_handle(Vec3::new(0.0, h * 0.86, front_z + 0.015), w * 0.65),
        ],
        Kind::ElectricOven => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.78, h * 0.56, 0.016),
                Vec3::new(0.0, h * 0.42, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.86, h * 0.17, 0.016),
                Vec3::new(0.0, h * 0.86, front_z),
                Surface::Appliance(1),
            ),
            appliance_handle(Vec3::new(0.0, h * 0.7, front_z + 0.02), w * 0.64),
        ],
        Kind::CoffeeMachine => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.62, h * 0.48, 0.02),
                Vec3::new(0.0, h * 0.57, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.7, 0.025, d * 0.42),
                Vec3::new(0.0, h * 0.07, d * 0.25),
                Surface::Appliance(1),
            ),
            // Twin coffee spouts make this silhouette unlike a microwave.
            Solid::axis(
                Vec3::new(0.025, h * 0.18, 0.025),
                Vec3::new(-w * 0.1, h * 0.34, front_z + 0.025),
                Surface::Handle,
            ),
            Solid::axis(
                Vec3::new(0.025, h * 0.18, 0.025),
                Vec3::new(w * 0.1, h * 0.34, front_z + 0.025),
                Surface::Handle,
            ),
            Solid::axis(
                Vec3::new(w * 0.48, h * 0.12, d * 0.5),
                Vec3::new(0.0, h * 0.94, -d * 0.08),
                Surface::Appliance(1),
            ),
        ],
        Kind::Microwave => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.7, h * 0.7, 0.016),
                Vec3::new(-w * 0.09, h * 0.5, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.12, h * 0.68, 0.016),
                Vec3::new(w * 0.39, h * 0.5, front_z),
                Surface::Appliance(1),
            ),
            appliance_handle(Vec3::new(w * 0.27, h * 0.5, front_z + 0.02), h * 0.48),
        ],
        Kind::SquareSink => rectangular_sink(size, 1),
        Kind::RoundSink => round_sink(size),
        Kind::DoubleSink => rectangular_sink(size, 2),
        Kind::SinkFaucet => {
            let tube = w.min(d).min(0.045).max(0.018);
            vec![
                Solid::axis(
                    Vec3::new(tube * 2.2, 0.025, tube * 2.2),
                    Vec3::new(0.0, 0.0125, 0.0),
                    Surface::ApplianceRound(1),
                ),
                Solid::axis(
                    Vec3::new(tube, h * 0.82, tube),
                    Vec3::new(0.0, h * 0.41, 0.0),
                    Surface::ApplianceRound(1),
                ),
                Solid::axis(
                    Vec3::new(tube, tube, d * 0.72),
                    Vec3::new(0.0, h * 0.82, d * 0.28),
                    Surface::ApplianceRound(1),
                ),
                Solid::axis(
                    Vec3::new(tube, h * 0.18, tube),
                    Vec3::new(0.0, h * 0.75, d * 0.64),
                    Surface::ApplianceRound(1),
                ),
            ]
        }
        Kind::CentralHeatingBoiler => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.42, h * 0.13, 0.018),
                Vec3::new(0.0, h * 0.28, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.74, 0.025, 0.025),
                Vec3::new(0.0, h * 0.1, front_z + 0.015),
                Surface::Handle,
            ),
            // Visible service pipes under the boiler.
            Solid::axis(
                Vec3::new(0.025, h * 0.2, 0.025),
                Vec3::new(-w * 0.22, -h * 0.1, 0.0),
                Surface::ApplianceRound(1),
            ),
            Solid::axis(
                Vec3::new(0.025, h * 0.2, 0.025),
                Vec3::new(w * 0.22, -h * 0.1, 0.0),
                Surface::ApplianceRound(1),
            ),
        ],
        Kind::AirConditioner => {
            let mut parts = vec![
                body,
                Solid::axis(
                    Vec3::new(w * 0.88, h * 0.18, 0.018),
                    Vec3::new(0.0, h * 0.18, front_z),
                    Surface::Appliance(2),
                ),
            ];
            for index in 0..5 {
                let x = -w * 0.32 + index as f32 * w * 0.16;
                parts.push(Solid::axis(
                    Vec3::new(0.012, h * 0.13, 0.022),
                    Vec3::new(x, h * 0.18, front_z + 0.014),
                    Surface::Appliance(1),
                ));
            }
            parts
        }
        _ => vec![body],
    }
}

/// A long bar handle, as on a fridge or oven door.
fn appliance_handle(at: Vec3, length: f32) -> Solid {
    Solid::axis(
        Vec3::new(length.max(0.04), 0.018, 0.018),
        at,
        Surface::Handle,
    )
}
