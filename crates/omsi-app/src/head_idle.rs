//! The part of the driver's head that moves when nothing else does.
//!
//! A person standing still does not hold their head still: it breathes up and down, shifts
//! its weight slowly from one side to the other and leans a little, and it wanders about the
//! middle while doing it. That is what this is - the sway of a head that has nothing to react
//! to, so the view is never perfectly frozen while the bus stands at a stop and the player
//! touches nothing.
//!
//! Each of those movements has its own slow rate, and none of the rates is a multiple of
//! another, so the pattern never lands on itself and the head does not look like a metronome.
//! The sideways sway and the lean have to be periodic: a movement that only drifts one way and
//! stops reads as nothing at all, however far it goes - the eye gives up on a slow uniform
//! slide (the same reason a bus climbing onto a bridge looks level). The long drift is still
//! there underneath each of them, in a smaller share, so that nothing repeats exactly.
//!
//! The vertical movement is the largest - and the one that is really noticed, because it is
//! the breath. The fore and aft movement is the smallest: from the driver's seat it runs along
//! the line of sight, so moving by it changes almost nothing on the screen; it is here for the
//! near field and for the mirrors, which the eye is asked for.
//!
//! It is *added* to what the bus's own motion does (OMSI's head movement, `Player::move_head`):
//! where that is thrown by accelerations and is therefore nothing at a standstill, this goes on
//! of its own. With the setting at zero (`Settings::head_idle`) it is exactly nothing: the step
//! puts the head back to the middle and no other code has to ask.

use glam::Vec3;

/// How far the head wanders at full strength: metres in the bus's frame (across, along, up).
const SIDE_M: f32 = 0.010;
const FORE_M: f32 = 0.003;
const UP_M: f32 = 0.014;

/// The turn it adds to the view at full strength, in degrees.
const YAW_DEG: f32 = 0.30;
const PITCH_DEG: f32 = 0.18;
const ROLL_DEG: f32 = 0.30;

/// The periods of the movements, in seconds: a breath (about fifteen a minute), the weight
/// going from one foot to the other, the body's lean, and the long wander of a standing body.
const BREATH_S: f32 = 4.1;
const WEIGHT_S: f32 = 6.4;
const LEAN_S: f32 = 5.7;
const WANDER_S: f32 = 13.1;

/// Where the head is and how it is turned on its own at one instant.
///
/// `offset` is in the bus's frame, so it is turned with the bus like the head that OMSI's own
/// head movement moves; the angles are what the view is turned by, in the same degrees and the
/// same sense as a head tracker's pose (a positive yaw turns left, as `finish` in `app_events`
/// adds it to the camera's yaw).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct HeadIdle {
    pub offset: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    /// Seconds of sway so far: it is what makes the movement smooth and a picture at a given
    /// time the same whatever the frame rate was.
    time: f32,
}

impl HeadIdle {
    /// Move the head on by `dt` seconds at `strength` (0 = off, 1 = the sway above).
    ///
    /// A step longer than this one is held back (a load hitch must not throw the head), and the
    /// whole movement is a function of the elapsed time alone, so it does not depend on how
    /// often it is asked.
    pub(crate) fn step(&mut self, dt: f32, strength: f32) {
        let strength = strength.clamp(0.0, 1.0);
        if strength <= 0.0 {
            *self = Self::default();
            return;
        }
        self.time += dt.clamp(0.0, 0.1);
        let t = self.time;
        self.offset = Vec3::new(
            // the weight from one foot to the other: the sideways sway, slow enough not to be
            // a step, and with a drift under it so that it never comes back to the same place
            SIDE_M * strength * (wave(t, WEIGHT_S, 0.35) * 0.75 + wander(t, WANDER_S, 12) * 0.25),
            // fore and aft: the smallest of the three (see the note at the top of the file)
            FORE_M * strength * wander(t, WANDER_S, 13),
            // the breath, the one movement that is really periodic, and the largest
            UP_M * strength * (wave(t, BREATH_S, 0.0) * 0.6 + wander(t, BREATH_S, 14) * 0.4),
        );
        self.yaw = YAW_DEG * strength * wander(t, WANDER_S, 15);
        self.pitch = PITCH_DEG * strength * wander(t, WEIGHT_S, 16);
        // the lean: a horizon only just tilting, at its own rate
        self.roll = ROLL_DEG * strength * (wave(t, LEAN_S, 0.6) * 0.7 + wander(t, WANDER_S, 17) * 0.3);
    }

    /// The head is where it began: nothing is added to the view at all.
    pub(crate) fn is_still(&self) -> bool {
        *self == Self::default()
    }
}

/// A slow, even wave in -1..1 that depends on the time alone. `phase` is a share of the
/// period, so that several waves do not all start from the same point.
fn wave(t: f32, period: f32, phase: f32) -> f32 {
    (std::f32::consts::TAU * (t / period + phase)).sin()
}

/// A smooth wander in -1..1 that depends on the time alone: one value every `period` seconds,
/// drawn from the cell's number and the seed, the two sides joined with a cubic so there are no
/// corners and no jitter. No state is kept, so asking for an earlier moment gives it back.
fn wander(t: f32, period: f32, seed: u32) -> f32 {
    let k = (t / period).floor();
    let f = t / period - k;
    let a = cell(k as i32, seed);
    let b = cell(k as i32 + 1, seed);
    let s = f * f * (3.0 - 2.0 * f);
    (a + (b - a) * s) * 2.0 - 1.0
}

/// A stable value in 0..1 for one cell of a wander: the same cell always gives the same value,
/// on every machine and in every run.
fn cell(index: i32, seed: u32) -> f32 {
    let mut h = (index as u32).wrapping_mul(0x9E37_79B9) ^ seed.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h >> 8) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sway after `seconds` of steps of `dt`, as the game would have made it.
    fn after(dt: f32, seconds: f32, strength: f32) -> HeadIdle {
        let mut head = HeadIdle::default();
        for _ in 0..(seconds / dt).round() as u32 {
            head.step(dt, strength);
        }
        head
    }

    #[test]
    fn at_zero_strength_the_head_is_exactly_still() {
        let head = after(1.0 / 60.0, 30.0, 0.0);
        assert!(head.is_still(), "{head:?}");
        assert_eq!(head, HeadIdle::default());
    }

    #[test]
    fn the_sway_does_not_depend_on_the_frame_rate() {
        // (a tenth of a second is the longest step the sway takes whole, see below)
        let slow = after(0.1, 20.0, 1.0);
        let fast = after(1.0 / 60.0, 20.0, 1.0);
        assert!((slow.offset - fast.offset).length() < 0.0005, "{slow:?} {fast:?}");
        assert!((slow.yaw - fast.yaw).abs() < 0.01 && (slow.roll - fast.roll).abs() < 0.01, "{slow:?} {fast:?}");
    }

    #[test]
    fn the_head_stays_where_a_head_can_be() {
        let mut head = HeadIdle::default();
        for _ in 0..(180.0 * 60.0) as u32 {
            head.step(1.0 / 60.0, 1.0);
            assert!(head.offset.x.abs() <= SIDE_M + 0.001, "{head:?}");
            assert!(head.offset.y.abs() <= FORE_M + 0.001, "{head:?}");
            assert!(head.offset.z.abs() <= UP_M + 0.001, "{head:?}");
            assert!(head.yaw.abs() <= YAW_DEG + 0.01 && head.pitch.abs() <= PITCH_DEG + 0.01 && head.roll.abs() <= ROLL_DEG + 0.01, "{head:?}");
        }
    }

    #[test]
    fn the_head_is_never_frozen_and_never_drifts_away() {
        let mut head = HeadIdle::default();
        let (mut low, mut high, mut sum) = (f32::MAX, f32::MIN, 0.0);
        let frames = 300.0 * 60.0;
        for _ in 0..frames as u32 {
            head.step(1.0 / 60.0, 1.0);
            low = low.min(head.offset.x);
            high = high.max(head.offset.x);
            sum += head.offset.x;
        }
        // it goes somewhere: at least a third of the way across its own range
        assert!(high - low > SIDE_M, "range {low}..{high}");
        // and it stays around the middle: no walking off to one side
        assert!((sum / frames as f32).abs() < SIDE_M * 0.25, "mean {}", sum / frames as f32);
    }

    #[test]
    fn the_sideways_sway_comes_back() {
        // (a movement that only drifts one way reads as nothing at all, however far it goes:
        // the sideways sway has to turn round again and again for the eye to catch it)
        let mut head = HeadIdle::default();
        let (mut changes, mut before) = (0, 0.0f32);
        for _ in 0..(60.0 * 60.0) as u32 {
            head.step(1.0 / 60.0, 1.0);
            if before != 0.0 && head.offset.x.signum() != before.signum() {
                changes += 1;
            }
            before = head.offset.x;
        }
        assert!(changes >= 10, "the sideways sway turned round {changes} times in a minute");
    }

    #[test]
    fn the_strength_is_how_much_of_the_sway_there_is() {
        for t in [1.0, 3.7, 9.2, 25.0] {
            let full = after(1.0 / 60.0, t, 1.0);
            let half = after(1.0 / 60.0, t, 0.5);
            assert!((half.offset - full.offset * 0.5).length() < 1e-5, "{t}: {half:?} {full:?}");
            assert!((half.roll - full.roll * 0.5).abs() < 1e-4, "{t}: {half:?} {full:?}");
        }
    }

    #[test]
    fn a_long_frame_does_not_throw_the_head() {
        let mut jumped = HeadIdle::default();
        jumped.step(2.0, 1.0);
        let patient = after(0.1, 0.1, 1.0);
        assert!((jumped.offset - patient.offset).length() < 0.002, "{jumped:?} {patient:?}");
    }
}
