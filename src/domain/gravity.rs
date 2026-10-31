//! Accelerometer calibration and gravity-based roll estimation.
//!
//! Body frame of the LG AN-MR200 remote (fitted from real static poses, |g| = 1.000 ± 0.002):
//! - `accel[0]` → X, longitudinal (nose): −1g pointing at the ceiling, +1g pointing at the floor.
//! - `accel[1]` → Y, lateral: +1g rolled 90° right, −1g rolled 90° left.
//! - `accel[2]` → Z, normal to the buttons: −1g held normally, +1g upside down.
//!
//! Roll is `atan2(Y, −Z)`. Pitch scales Y and Z by the same `cos(pitch)`, so the angle is
//! independent of where the remote is pointing, as long as it is not near-vertical.

/// Per-axis offset and counts-per-g for converting raw ADC readings into g.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccelCalibration {
    pub offset: [f32; 3],
    pub counts_per_g: [f32; 3],
}

impl AccelCalibration {
    pub const LG_AN_MR200: Self = Self {
        offset: [2055.0, 2012.0, 2180.0],
        counts_per_g: [1018.0, 1037.0, 1037.0],
    };

    pub fn to_g(&self, raw: [u16; 3]) -> [f32; 3] {
        std::array::from_fn(|i| (raw[i] as f32 - self.offset[i]) / self.counts_per_g[i])
    }
}

impl Default for AccelCalibration {
    fn default() -> Self {
        Self::LG_AN_MR200
    }
}

/// Tracks the gravity direction in the remote's transverse (Y/Z) plane and derives roll.
///
/// Unlike naive velocity-gated filters that freeze roll estimation whenever the cursor moves,
/// this estimator tracks roll continuously during active motion by:
/// 1. Subtracting wrist-pivot tangential acceleration (`d(gyro)/dt`) from the lateral/normal
///    accelerometer axes (`Y` and `Z`), since rigid-body tangential acceleration is proportional
///    to angular acceleration rather than angular velocity.
/// 2. Gating on high-frequency jerk (sudden whip reversals / tremor) and total `|g|` deviation
///    rather than angular velocity, allowing smooth simultaneous wrist rotation and cursor movement.
#[derive(Clone, Copy, Debug)]
pub struct RollEstimator {
    calibration: AccelCalibration,
    grav_y: f32,
    grav_z: f32,
    prev_dx: f32,
    prev_dy: f32,
    has_prev: bool,
    smooth_ddx: f32,
    smooth_ddy: f32,
    jerk_envelope: f32,
}

impl RollEstimator {
    const BASE_BLEND_MIN: f32 = 0.12;
    const BASE_BLEND_ADAPTIVE: f32 = 0.28;
    const MAGNITUDE_TOLERANCE_G: f32 = 0.14;
    const JERK_FREEZE: f32 = 16.0;
    const JERK_DECAY: f32 = 0.72;
    const ACCEL_SMOOTH_ALPHA: f32 = 0.25;
    /// Empirical wrist-pivot tangential coupling from angular acceleration (g per count/frame).
    const TANGENTIAL_COEFF_Y: f32 = 0.040;
    const TANGENTIAL_COEFF_Z: f32 = 0.020;
    /// Below this transverse gravity the remote points near-vertically and roll is undefined.
    const MIN_TRANSVERSE_G: f32 = 0.45;

    pub fn new(calibration: AccelCalibration) -> Self {
        Self {
            calibration,
            grav_y: 0.0,
            grav_z: -1.0,
            prev_dx: 0.0,
            prev_dy: 0.0,
            has_prev: false,
            smooth_ddx: 0.0,
            smooth_ddy: 0.0,
            jerk_envelope: 0.0,
        }
    }

    /// Feeds one accelerometer sample along with signed 2D gyro deltas (`gyro_dx`, `gyro_dy` in raw counts).
    /// Continuously tracks roll even while the user is actively moving the cursor.
    pub fn update_motion(&mut self, raw_accel: [u16; 3], gyro_dx: f32, gyro_dy: f32) {
        let (ddx, ddy) = if self.has_prev {
            (gyro_dx - self.prev_dx, gyro_dy - self.prev_dy)
        } else {
            self.has_prev = true;
            (0.0, 0.0)
        };
        self.prev_dx = gyro_dx;
        self.prev_dy = gyro_dy;

        // High-frequency jerk is the deviation of angular acceleration from its smoothed trend.
        let j_x = ddx - self.smooth_ddx;
        let j_y = ddy - self.smooth_ddy;
        self.smooth_ddx += Self::ACCEL_SMOOTH_ALPHA * j_x;
        self.smooth_ddy += Self::ACCEL_SMOOTH_ALPHA * j_y;

        let hf_jerk = j_x.hypot(j_y);
        self.jerk_envelope = hf_jerk.max(self.jerk_envelope * Self::JERK_DECAY);

        let [x, y, z] = self.calibration.to_g(raw_accel);

        // Compensate tangential acceleration induced by wrist/elbow angular acceleration.
        let yc = y + Self::TANGENTIAL_COEFF_Y * self.smooth_ddx;
        let zc = z - Self::TANGENTIAL_COEFF_Z * self.smooth_ddy;

        let trans = yc.hypot(zc);
        if trans < Self::MIN_TRANSVERSE_G {
            return;
        }

        let ny = yc / trans;
        let nz = zc / trans;

        let mag_raw = (x * x + y * y + z * z).sqrt();
        let mag_comp = (x * x + yc * yc + zc * zc).sqrt();
        let mag_err = (mag_raw - 1.0).abs().min((mag_comp - 1.0).abs());
        let magnitude_trust = (1.0 - mag_err / Self::MAGNITUDE_TOLERANCE_G).max(0.0);
        let jerk_trust = (1.0 - self.jerk_envelope / Self::JERK_FREEZE).max(0.0);
        let trans_trust = ((trans - Self::MIN_TRANSVERSE_G) / 0.30).min(1.0);

        let dot = (ny * self.grav_y + nz * self.grav_z).clamp(-1.0, 1.0);
        let angle_diff = dot.acos();
        let base_blend = Self::BASE_BLEND_MIN
            + Self::BASE_BLEND_ADAPTIVE * (angle_diff / 0.45).min(1.0);

        let blend = base_blend * magnitude_trust * jerk_trust * trans_trust;

        self.grav_y += (ny - self.grav_y) * blend;
        self.grav_z += (nz - self.grav_z) * blend;

        let norm = self.grav_y.hypot(self.grav_z);
        if norm > 1e-5 {
            self.grav_y /= norm;
            self.grav_z /= norm;
        }
    }

    /// Feeds one accelerometer sample; `gyro_activity` is the angular-rate magnitude in raw counts.
    pub fn update(&mut self, raw_accel: [u16; 3], gyro_activity: f32) {
        self.update_motion(raw_accel, gyro_activity, 0.0);
    }

    /// Roll in radians, positive when rolled to the right.
    pub fn roll(&self) -> f32 {
        self.grav_y.atan2(-self.grav_z)
    }

    pub fn roll_deg(&self) -> f32 {
        self.roll().to_degrees()
    }
}

impl Default for RollEstimator {
    fn default() -> Self {
        Self::new(AccelCalibration::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mean raw readings captured from the real remote in each static pose.
    const POINT_NORMAL: [u16; 3] = [2050, 2023, 1143];
    const ROLL_RIGHT_90: [u16; 3] = [2004, 3027, 1967];
    const ROLL_LEFT_90: [u16; 3] = [2001, 980, 2122];
    const ROLL_RIGHT_45: [u16; 3] = [2006, 2559, 1303];
    const ROLL_LEFT_45: [u16; 3] = [2006, 1321, 1406];
    const UPSIDE_DOWN: [u16; 3] = [1916, 1931, 3204];
    const POINT_CEILING: [u16; 3] = [1042, 2086, 2124];
    const POINT_FLOOR: [u16; 3] = [3069, 1974, 2100];

    fn settle(est: &mut RollEstimator, raw: [u16; 3]) {
        for _ in 0..200 {
            est.update(raw, 0.0);
        }
    }

    #[test]
    fn calibration_yields_one_g_in_every_pose() {
        let cal = AccelCalibration::LG_AN_MR200;
        for raw in [POINT_NORMAL, ROLL_RIGHT_90, ROLL_LEFT_90, ROLL_RIGHT_45, ROLL_LEFT_45, UPSIDE_DOWN, POINT_CEILING, POINT_FLOOR] {
            let [x, y, z] = cal.to_g(raw);
            let mag = (x * x + y * y + z * z).sqrt();
            assert!((mag - 1.0).abs() < 0.02, "|g| = {mag} for {raw:?}");
        }
    }

    #[test]
    fn roll_matches_real_poses() {
        let cases = [
            (POINT_NORMAL, 0.0, 3.0),
            (ROLL_RIGHT_90, 80.0, 12.0),
            (ROLL_LEFT_90, -87.0, 12.0),
            (ROLL_RIGHT_45, 32.0, 15.0),
            (ROLL_LEFT_45, -42.0, 15.0),
        ];
        for (raw, expected, tol) in cases {
            let mut est = RollEstimator::default();
            settle(&mut est, raw);
            let roll = est.roll_deg();
            assert!((roll - expected).abs() < tol, "expected ~{expected}°, got {roll}° for {raw:?}");
        }
    }

    #[test]
    fn upside_down_is_near_180() {
        let mut est = RollEstimator::default();
        settle(&mut est, UPSIDE_DOWN);
        assert!(est.roll_deg().abs() > 170.0, "got {}", est.roll_deg());
    }

    #[test]
    fn pointing_up_or_down_does_not_change_roll() {
        let mut est = RollEstimator::default();
        settle(&mut est, POINT_NORMAL);
        let before = est.roll_deg();
        settle(&mut est, POINT_CEILING);
        settle(&mut est, POINT_FLOOR);
        assert!((est.roll_deg() - before).abs() < 1.0, "roll drifted to {}", est.roll_deg());
    }

    #[test]
    fn linear_acceleration_during_motion_is_ignored() {
        let mut est = RollEstimator::default();
        settle(&mut est, POINT_NORMAL);
        for i in 0..100 {
            let lateral = if i % 20 < 10 { 2600 } else { 1450 };
            let rate = if i % 20 == 0 || i % 20 == 10 { 0.0 } else { 150.0 };
            est.update([2050, lateral, 1143], rate);
        }
        assert!(est.roll_deg().abs() < 3.0, "roll corrupted to {}", est.roll_deg());
    }

    #[test]
    fn roll_updates_dynamically_during_continuous_cursor_movement() {
        let mut est = RollEstimator::default();
        settle(&mut est, POINT_NORMAL);

        // Continuously sweep the cursor (rate ~ 60..80 counts) while rolling 90° right without stopping
        for i in 0..35 {
            let dx = 70.0 * (i as f32 * 0.12).sin();
            let dy = 50.0 * (i as f32 * 0.12).cos();
            est.update_motion(ROLL_RIGHT_90, dx, dy);
        }
        assert!(
            (est.roll_deg() - 80.0).abs() < 12.0,
            "roll should converge to ~80° while moving continuously, got {}",
            est.roll_deg()
        );
    }
}
