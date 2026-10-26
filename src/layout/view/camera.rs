//! The orbit camera's angles and the presets the sidebar buttons use.
//!
//! The camera circles a `focus` point at a `distance`. `yaw` turns it around
//! the vertical, and `pitch` raises it above the floor. Input that changes
//! these numbers lives in `edit/camera.rs`.

use bevy::prelude::*;

use crate::layout::model::Room;

/// Angles for the modelling camera. The transform on the same entity is
/// rewritten from these every frame, so the sidebar can change the view by
/// editing the angles alone.
#[derive(Component)]
pub struct OrbitCamera {
    pub focus: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl OrbitCamera {
    /// The starting camera: from the front-left corner, framing the whole room.
    pub fn looking_at_room(room: &Room) -> Self {
        let mut camera = Self {
            focus: Vec3::ZERO,
            // From the front-left corner, a little above the cabinets.
            yaw: 2.25,
            pitch: 0.62,
            distance: 8.0,
        };
        camera.frame_room(room);
        camera
    }

    /// Aim at the middle of the room, far enough back to see all of it.
    pub fn frame_room(&mut self, room: &Room) {
        let size = room.size_meters();
        self.focus = Vec3::new(size.x * 0.5, size.y * 0.28, size.z * 0.45);
        self.distance = size.x.hypot(size.z).max(2.0) * 0.95;
    }

    /// Aim at one piece (or a group), close enough to fill the view.
    pub fn frame_piece(&mut self, base: Vec3, size: Vec3) {
        self.focus = base + Vec3::Y * size.y * 0.5;
        self.distance = size.x.max(size.y).max(size.z).max(0.4) * 3.2;
    }

    /// From the front-left corner, slightly above.
    pub fn corner_view(&mut self) {
        self.yaw = 2.25;
        self.pitch = 0.62;
    }

    /// Straight at the back wall.
    pub fn front_view(&mut self) {
        // yaw of a quarter turn puts the camera on the +Z side, looking back
        // toward the back wall.
        self.yaw = std::f32::consts::FRAC_PI_2;
        self.pitch = 0.18;
    }

    /// Almost straight down, like a floor plan.
    pub fn top_view(&mut self) {
        // Steep, but not straight down: a vertical look direction makes the
        // camera's "up" undefined.
        self.yaw = std::f32::consts::FRAC_PI_2;
        self.pitch = 1.42;
    }

    /// The camera transform for the current angles.
    pub fn pose(&self) -> Transform {
        Transform::from_translation(self.eye()).looking_at(self.focus, Vec3::Y)
    }

    /// Camera position: `distance` away from `focus`, at `yaw` and `pitch`.
    fn eye(&self) -> Vec3 {
        let pitch = self.pitch.clamp(0.08, 1.45);
        let horizontal = self.distance * pitch.cos();
        self.focus
            + Vec3::new(
                horizontal * self.yaw.cos(),
                self.distance * pitch.sin(),
                horizontal * self.yaw.sin(),
            )
    }
}
