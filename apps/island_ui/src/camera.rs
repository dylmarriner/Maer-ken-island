//! Where the eye is.
//!
//! An orbit camera over the map rather than a flight simulator: the thing
//! people do with this island is look at a place on it and turn around
//! that place. The focus is a point in the regional frame, and the camera
//! sits on a sphere around it.
//!
//! Pure arithmetic, so it can be tested without a window, which on a
//! machine with no GPU is the only way it can be tested at all. The Bevy
//! side reads [`Orbit::eye`] and [`Orbit::focus`] and does nothing else.

use crate::scene::{Point, REGIONAL_METRES_PER_UNIT};

/// How far out the camera may go, in regional units.
///
/// The island is 240 units across; 600 holds all of it with room around.
/// Further than this and the far plane starts costing depth precision for
/// a view that shows nothing new.
pub const MAX_DISTANCE: f32 = 600.0;

/// And how close. Below this the near plane starts clipping the ground the
/// camera is looking at.
pub const MIN_DISTANCE: f32 = 0.02;

/// Hard limits on pitch, a hair off the poles.
///
/// Exactly overhead the up vector and the view direction are parallel and
/// the look-at matrix is undefined -- the view spins on its own axis for
/// no input, which reads as the island suddenly rotating. Stopping just
/// short costs nothing anybody can see.
pub const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.01;
pub const MIN_PITCH: f32 = -MAX_PITCH;

/// The camera's state: what it is looking at, and from where.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
    pub focus: Point,
    /// Distance from the focus, in regional units.
    pub distance: f32,
    /// Radians east of north.
    pub yaw: f32,
    /// Radians above the horizon. Positive looks down.
    pub pitch: f32,
}

impl Default for Orbit {
    /// The whole island, from the south-east, high enough to see its
    /// shape and low enough that the relief reads.
    fn default() -> Self {
        Self {
            focus: Point::new(0.0, 0.0, 0.0),
            distance: 320.0,
            yaw: std::f32::consts::FRAC_PI_4,
            pitch: 0.9,
        }
    }
}

impl Orbit {
    /// Where the eye is, in the regional frame.
    pub fn eye(&self) -> Point {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Point::new(
            self.focus.x + self.distance * cos_pitch * sin_yaw,
            self.focus.y + self.distance * sin_pitch,
            self.focus.z + self.distance * cos_pitch * cos_yaw,
        )
    }

    /// Turn, clamping pitch off the poles.
    pub fn turn(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.yaw = (self.yaw + yaw_delta).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + pitch_delta).clamp(MIN_PITCH, MAX_PITCH);
    }

    /// Zoom by a factor, clamped.
    ///
    /// Multiplied rather than added, which is the whole difference between
    /// a zoom that works across six orders of magnitude and one that
    /// does not: at 300 units a step of one is invisible, and at 0.05 the
    /// same step is through the ground.
    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    /// Slide the focus across the ground, in the camera's own directions,
    /// scaled by how far out it is -- so a drag moves the same fraction of
    /// the screen whatever the zoom.
    pub fn pan(&mut self, right: f32, forward: f32) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let scale = self.distance;
        self.focus = Point::new(
            self.focus.x + (right * cos_yaw - forward * sin_yaw) * scale,
            self.focus.y,
            self.focus.z + (-right * sin_yaw - forward * cos_yaw) * scale,
        );
    }

    /// Frame a place: look at it from a sensible distance.
    pub fn look_at(&mut self, focus: Point, distance: f32) {
        self.focus = focus;
        self.distance = distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    /// How wide the view is on the ground, in metres, for a scale bar that
    /// says what the picture is showing.
    pub fn across_m(&self, vertical_fov: f32, aspect: f32) -> f64 {
        let height = 2.0 * self.distance * (vertical_fov / 2.0).tan();
        f64::from(height * aspect) * REGIONAL_METRES_PER_UNIT
    }

    /// Near and far planes that hold this view without wasting depth.
    ///
    /// Tied to the distance rather than fixed, and that is the difference
    /// between an island that renders and one that z-fights: a near plane
    /// of 0.1 with a far plane of 2,000 spends almost all of its depth
    /// precision in the first few units, so at a four-metre room the
    /// ground and the floor flicker against each other.
    pub fn clip_planes(&self) -> (f32, f32) {
        let near = (self.distance * 0.01).clamp(0.000_1, 10.0);
        let far = (self.distance * 10.0).max(100.0);
        (near, far)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_view_holds_the_whole_island() {
        // 2,401 km at 10 km per unit is 240 units across. A 320-unit
        // distance at a 45 degree field of view holds it.
        let orbit = Orbit::default();
        let across = orbit.across_m(std::f32::consts::FRAC_PI_4, 16.0 / 9.0);
        assert!(
            across > 2_401_000.0,
            "the default view is only {:.0} km across",
            across / 1000.0
        );
    }

    #[test]
    fn the_eye_is_the_distance_away_whichever_way_it_faces() {
        let mut orbit = Orbit::default();
        for _ in 0..20 {
            orbit.turn(0.3, 0.07);
            let eye = orbit.eye();
            assert!(
                (eye.distance(&orbit.focus) - orbit.distance).abs() < 1e-2,
                "{eye:?} is not {} from {:?}",
                orbit.distance,
                orbit.focus
            );
        }
    }

    #[test]
    fn pitch_stops_short_of_the_poles() {
        // Exactly overhead, the look-at matrix is undefined and the view
        // spins on its own axis for no input.
        let mut orbit = Orbit::default();
        orbit.turn(0.0, 100.0);
        assert!(orbit.pitch <= MAX_PITCH, "{}", orbit.pitch);
        assert!(orbit.pitch < std::f32::consts::FRAC_PI_2);
        orbit.turn(0.0, -100.0);
        assert!(orbit.pitch >= MIN_PITCH, "{}", orbit.pitch);
    }

    #[test]
    fn yaw_wraps_instead_of_growing_without_end() {
        let mut orbit = Orbit::default();
        for _ in 0..1000 {
            orbit.turn(1.0, 0.0);
        }
        assert!(
            orbit.yaw >= 0.0 && orbit.yaw < std::f32::consts::TAU,
            "{}",
            orbit.yaw
        );
    }

    #[test]
    fn zooming_is_multiplicative_so_it_works_at_every_scale() {
        // The island is 240 units and a bedroom is four metres, which is
        // 0.0004 units. A linear zoom cannot serve both.
        let mut orbit = Orbit::default();
        for _ in 0..200 {
            orbit.zoom(0.9);
        }
        assert_eq!(orbit.distance, MIN_DISTANCE, "it should reach the floor");
        for _ in 0..400 {
            orbit.zoom(1.1);
        }
        assert_eq!(orbit.distance, MAX_DISTANCE, "and the ceiling");
    }

    #[test]
    fn a_pan_moves_the_focus_the_way_the_camera_faces() {
        let mut orbit = Orbit {
            yaw: 0.0,
            ..Orbit::default()
        };
        let before = orbit.focus;
        orbit.pan(0.1, 0.0);
        assert!(orbit.focus.x > before.x, "right is east at yaw zero");
        assert_eq!(orbit.focus.y, before.y, "panning must not change height");

        // Turned a quarter turn, the same drag goes a different way.
        let mut turned = Orbit {
            yaw: std::f32::consts::FRAC_PI_2,
            ..Orbit::default()
        };
        turned.pan(0.1, 0.0);
        assert!(turned.focus.z < 0.0, "{:?}", turned.focus);
        // A hair, not zero: `sin` and `cos` of a quarter turn are not
        // exact in `f32`, and the residue scales with the distance --
        // 1.4 micrometres at 320 units, which is nothing on a frame where
        // one unit is ten kilometres.
        assert!(turned.focus.x.abs() < 1e-4, "{:?}", turned.focus);
    }

    #[test]
    fn a_pan_covers_the_same_screen_distance_at_every_zoom() {
        let mut near = Orbit {
            distance: 1.0,
            yaw: 0.0,
            ..Orbit::default()
        };
        let mut far = Orbit {
            distance: 100.0,
            yaw: 0.0,
            ..Orbit::default()
        };
        near.pan(0.1, 0.0);
        far.pan(0.1, 0.0);
        assert!(
            (far.focus.x / near.focus.x - 100.0).abs() < 1e-3,
            "a drag at 100 units should move 100x what it does at 1: {} and {}",
            far.focus.x,
            near.focus.x
        );
    }

    #[test]
    fn the_clip_planes_follow_the_zoom() {
        // A fixed near plane is what makes a four-metre room z-fight
        // while the whole island renders cleanly.
        let island = Orbit::default();
        let (near_far, far_far) = island.clip_planes();
        let room = Orbit {
            distance: 0.001,
            ..Orbit::default()
        };
        let (near_close, _) = room.clip_planes();
        assert!(
            near_close < near_far,
            "{near_close} is not nearer than {near_far}"
        );
        assert!(
            far_far > 240.0,
            "the far plane must hold the island: {far_far}"
        );
        // And the ratio stays sane, which is what depth precision
        // actually depends on.
        assert!(far_far / near_far < 2_000.0, "{}", far_far / near_far);
    }

    #[test]
    fn looking_at_a_place_clamps_rather_than_accepting_anything() {
        let mut orbit = Orbit::default();
        orbit.look_at(Point::new(1.0, 2.0, 3.0), 1e9);
        assert_eq!(orbit.focus, Point::new(1.0, 2.0, 3.0));
        assert_eq!(orbit.distance, MAX_DISTANCE);
        orbit.look_at(Point::new(0.0, 0.0, 0.0), -5.0);
        assert_eq!(orbit.distance, MIN_DISTANCE);
    }
}
