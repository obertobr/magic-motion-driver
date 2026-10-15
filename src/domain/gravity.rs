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
/// The accelerometer also sees hand acceleration, so a sample only updates the estimate in
/// proportion to how trustworthy it is:
/// - its magnitude must be close to 1g (no linear acceleration), and
/// - the gyro must have been calm recently. A decaying peak-hold of gyro activity keeps the
///   estimate frozen through motion reversals, where angular rate crosses zero while linear
///   acceleration peaks.
#[derive(Clone, Copy, Debug)]
pub struct RollEstimator {
    calibration: AccelCalibration,
    grav_y: f32,
    grav_z: f32,
    activity_envelope: f32,
}

impl RollEstimator {
    const MAX_BLEND: f32 = 0.15;
    const MAGNITUDE_TOLERANCE_G: f32 = 0.15;
    const ACTIVITY_FREEZE: f32 = 20.0;
    const ACTIVITY_DECAY: f32 = 0.97;
    /// Below this transverse gravity the remote points near-vertically and roll is undefined.
    const MIN_TRANSVERSE_G: f32 = 0.5;

    pub fn new(calibration: AccelCalibration) -> Self {
        Self {
            calibration,
            grav_y: 0.0,
            grav_z: -1.0,
            activity_envelope: 0.0,
        }
    }

    /// Feeds one accelerometer sample; `gyro_activity` is the angular-rate magnitude in raw counts.
    pub fn update(&mut self, raw_accel: [u16; 3], gyro_activity: f32) {
        self.activity_envelope = gyro_activity.max(self.activity_envelope * Self::ACTIVITY_DECAY);

        let [x, y, z] = self.calibration.to_g(raw_accel);
        if y.hypot(z) < Self::MIN_TRANSVERSE_G {
            return;
        }

        let magnitude = (x * x + y * y + z * z).sqrt();
        let magnitude_trust = (1.0 - (magnitude - 1.0).abs() / Self::MAGNITUDE_TOLERANCE_G).max(0.0);
        let calm_trust = (1.0 - self.activity_envelope / Self::ACTIVITY_FREEZE).max(0.0);
        let blend = Self::MAX_BLEND * magnitude_trust * calm_trust;

        self.grav_y += (y - self.grav_y) * blend;
        self.grav_z += (z - self.grav_z) * blend;
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
}
