//! Pure gyro motion processing for mouse and camera control.
//!
//! Features:
//! - Sub-pixel accumulator: preserves sub-pixel precision across frames so slow
//!   aiming in games and fine cursor positioning never drop movement.
//! - Deadzone: eliminates resting tremor and sensor noise.
//! - Power acceleration curve: gentle 1:1 precision at slow speeds, responsive
//!   turns during fast flicks.
//! - Low-pass filter (smoothing): exponential moving average for jitter-free tracking.
//! - Invert Y option for flight and camera preferences.

use crate::domain::gravity::RollEstimator;

/// Motion processor settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionSettings {
    /// Overall sensitivity multiplier. Default ~ 0.35.
    pub sensitivity: f32,
    /// Separate X axis multiplier (e.g. 1.0).
    pub sensitivity_x: f32,
    /// Separate Y axis multiplier (e.g. 1.0).
    pub sensitivity_y: f32,
    /// Deadzone threshold around center (in raw ADC counts, e.g. 1.5).
    pub deadzone: f32,
    /// Acceleration exponent (1.0 = linear, 1.2..1.5 = progressive curve).
    pub acceleration: f32,
    /// Smoothing factor in [0.0, 1.0) (0.0 = raw response, 0.3 = smooth).
    pub smoothing: f32,
    /// Invert horizontal axis.
    pub invert_x: bool,
    /// Invert vertical axis.
    pub invert_y: bool,
    /// Gravity-aware roll compensation (up is always up regardless of remote tilt).
    pub roll_compensation: bool,
}

impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.35,
            sensitivity_x: 1.0,
            sensitivity_y: 1.0,
            deadzone: 1.5,
            acceleration: 1.15,
            smoothing: 0.25,
            invert_x: false,
            invert_y: false,
            roll_compensation: true,
        }
    }
}

/// Zero-allocation, stateful motion pipeline.
#[derive(Clone, Copy, Debug)]
pub struct MotionProcessor {
    pub center_x: f32,
    pub center_y: f32,
    accum_x: f32,
    accum_y: f32,
    smooth_x: f32,
    smooth_y: f32,
    roll_estimator: RollEstimator,
    settings: MotionSettings,
    cursor_enabled: bool,
}

impl Default for MotionProcessor {
    fn default() -> Self {
        Self::new(MotionSettings::default())
    }
}

impl MotionProcessor {
    pub const DEFAULT_CENTER: f32 = 537.0;

    pub fn new(settings: MotionSettings) -> Self {
        Self {
            center_x: Self::DEFAULT_CENTER,
            center_y: Self::DEFAULT_CENTER,
            accum_x: 0.0,
            accum_y: 0.0,
            smooth_x: 0.0,
            smooth_y: 0.0,
            roll_estimator: RollEstimator::default(),
            settings,
            cursor_enabled: true,
        }
    }

    pub fn set_settings(&mut self, settings: MotionSettings) {
        self.settings = settings;
    }

    pub fn settings(&self) -> &MotionSettings {
        &self.settings
    }

    pub fn set_cursor_enabled(&mut self, enabled: bool) {
        self.cursor_enabled = enabled;
        if !enabled {
            self.accum_x = 0.0;
            self.accum_y = 0.0;
            self.smooth_x = 0.0;
            self.smooth_y = 0.0;
        }
    }

    pub fn is_cursor_enabled(&self) -> bool {
        self.cursor_enabled
    }

    pub fn toggle_cursor(&mut self) -> bool {
        self.set_cursor_enabled(!self.cursor_enabled);
        self.cursor_enabled
    }

    /// Calibrates gyro center from a slice of resting frames.
    pub fn set_center(&mut self, center_x: f32, center_y: f32) {
        self.center_x = center_x;
        self.center_y = center_y;
        self.accum_x = 0.0;
        self.accum_y = 0.0;
        self.smooth_x = 0.0;
        self.smooth_y = 0.0;
    }

    /// Calibrates from multiple raw gyro readings (average).
    pub fn calibrate_samples(&mut self, samples: &[(u16, u16)]) {
        if samples.is_empty() {
            return;
        }
        let sum_x: f64 = samples.iter().map(|&(x, _)| x as f64).sum();
        let sum_y: f64 = samples.iter().map(|&(_, y)| y as f64).sum();
        let len = samples.len() as f64;
        self.set_center((sum_x / len) as f32, (sum_y / len) as f32);
    }

    /// Processes one gyro reading with gravity-aware roll compensation from the accelerometer.
    pub fn process_with_accel(
        &mut self,
        raw_gyro_x: u16,
        raw_gyro_y: u16,
        accel: [u16; 3],
    ) -> (i32, i32) {
        if !self.cursor_enabled {
            return (0, 0);
        }

        // 1. Raw delta from calibrated center.
        // Base sign: (center - raw) produces intuitive direction for the LG Hillcrest gyro.
        let mut raw_dx = self.center_x - raw_gyro_x as f32;
        let mut raw_dy = self.center_y - raw_gyro_y as f32;

        if self.settings.invert_x {
            raw_dx = -raw_dx;
        }
        if self.settings.invert_y {
            raw_dy = -raw_dy;
        }

        // 2. Deadzone: eliminate noise at rest
        let dz = self.settings.deadzone.max(0.0);
        if raw_dx.abs() <= dz {
            raw_dx = 0.0;
        } else {
            raw_dx -= raw_dx.signum() * dz;
        }

        if raw_dy.abs() <= dz {
            raw_dy = 0.0;
        } else {
            raw_dy -= raw_dy.signum() * dz;
        }

        // 3. Roll (gravity) compensation:
        let gyro_activity = (raw_dx * raw_dx + raw_dy * raw_dy).sqrt();
        self.roll_estimator.update(accel, gyro_activity);

        if self.settings.roll_compensation && (raw_dx != 0.0 || raw_dy != 0.0) {
            let eff_deg = self.effective_roll_deg();
            if eff_deg.abs() > 0.01 {
                let roll_rad = eff_deg.to_radians();
                let cos_phi = roll_rad.cos();
                let sin_phi = roll_rad.sin();

                let rot_dx = raw_dx * cos_phi - raw_dy * sin_phi;
                let rot_dy = raw_dx * sin_phi + raw_dy * cos_phi;

                raw_dx = rot_dx;
                raw_dy = rot_dy;
            }
        }

        if raw_dx == 0.0 && raw_dy == 0.0 {
            // Decay smoothing towards zero at rest
            self.smooth_x *= 0.5;
            self.smooth_y *= 0.5;
            return (0, 0);
        }

        // 4. Low-pass filter (smoothing)
        let alpha = self.settings.smoothing.clamp(0.0, 0.95);
        self.smooth_x = self.smooth_x * alpha + raw_dx * (1.0 - alpha);
        self.smooth_y = self.smooth_y * alpha + raw_dy * (1.0 - alpha);

        // 5. Sensitivity & Acceleration curve
        let mag = (self.smooth_x * self.smooth_x + self.smooth_y * self.smooth_y).sqrt();
        let accel_factor = if self.settings.acceleration > 1.0 && mag > 1.0 {
            mag.powf(self.settings.acceleration - 1.0).min(5.0)
        } else {
            1.0
        };

        let move_x = self.smooth_x * self.settings.sensitivity * self.settings.sensitivity_x * accel_factor;
        let move_y = self.smooth_y * self.settings.sensitivity * self.settings.sensitivity_y * accel_factor;

        // 6. Sub-pixel accumulator
        self.accum_x += move_x;
        self.accum_y += move_y;

        let out_x = self.accum_x.trunc() as i32;
        let out_y = self.accum_y.trunc() as i32;

        self.accum_x -= out_x as f32;
        self.accum_y -= out_y as f32;

        (out_x, out_y)
    }

    /// Convenience process wrapper using neutral gravity orientation (pointing forward, buttons up).
    pub fn process(&mut self, raw_gyro_x: u16, raw_gyro_y: u16) -> (i32, i32) {
        self.process_with_accel(raw_gyro_x, raw_gyro_y, [2050, 2023, 1143])
    }

    /// Returns the current estimated roll angle in degrees (-180° to +180°).
    pub fn current_roll_deg(&self) -> f32 {
        self.roll_estimator.roll_deg()
    }

    /// Effective roll angle applied to the cursor with smooth deadband for natural upright hand posture.
    pub fn effective_roll_deg(&self) -> f32 {
        let raw = self.current_roll_deg();
        let abs_d = raw.abs();
        if abs_d < 3.0 {
            0.0
        } else if abs_d < 12.0 {
            let t = (abs_d - 3.0) / 9.0;
            let factor = 3.0 * t * t - 2.0 * t * t * t;
            raw * factor
        } else {
            raw
        }
    }

    /// Resets sub-pixel accumulator and smoothing history.
    pub fn reset_history(&mut self) {
        self.accum_x = 0.0;
        self.accum_y = 0.0;
        self.smooth_x = 0.0;
        self.smooth_y = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_state_outputs_zero_delta() {
        let mut proc = MotionProcessor::default();
        proc.set_center(537.0, 537.0);

        for _ in 0..10 {
            assert_eq!(proc.process(537, 537), (0, 0));
        }
    }

    #[test]
    fn deadzone_suppresses_small_vibrations() {
        let mut proc = MotionProcessor::default();
        proc.set_center(537.0, 537.0);

        // Within deadzone (default 1.5)
        assert_eq!(proc.process(538, 536), (0, 0));
        assert_eq!(proc.process(536, 538), (0, 0));
    }

    #[test]
    fn subpixel_accumulates_slow_motion() {
        let mut settings = MotionSettings::default();
        settings.deadzone = 0.0;
        settings.smoothing = 0.0;
        settings.acceleration = 1.0;
        settings.sensitivity = 0.4; // 0.4 pixels per unit delta

        let mut proc = MotionProcessor::new(settings);
        proc.set_center(500.0, 500.0);

        // Base delta: center (500) - raw (499) = +1 unit -> 0.4 px per frame
        // Frame 1: 0.4 accum -> 0 out
        // Frame 2: 0.8 accum -> 0 out
        // Frame 3: 1.2 accum -> 1 out, 0.2 remaining
        let r1 = proc.process(499, 500);
        let r2 = proc.process(499, 500);
        let r3 = proc.process(499, 500);

        assert_eq!(r1, (0, 0));
        assert_eq!(r2, (0, 0));
        assert_eq!(r3, (1, 0));
    }

    #[test]
    fn invert_y_reverses_vertical() {
        let mut settings = MotionSettings::default();
        settings.deadzone = 0.0;
        settings.smoothing = 0.0;
        settings.acceleration = 1.0;
        settings.sensitivity = 1.0;
        settings.invert_y = true;

        let mut proc = MotionProcessor::new(settings);
        proc.set_center(500.0, 500.0);

        // Raw 490 -> base dy = 500 - 490 = +10. With invert_y = true -> dy = -10.
        let (_, dy) = proc.process(500, 490);
        assert!(dy < 0, "inverted Y should be negative for positive base delta");
    }

    #[test]
    fn invert_x_reverses_horizontal() {
        let mut settings = MotionSettings::default();
        settings.deadzone = 0.0;
        settings.smoothing = 0.0;
        settings.acceleration = 1.0;
        settings.sensitivity = 1.0;
        settings.invert_x = true;

        let mut proc = MotionProcessor::new(settings);
        proc.set_center(500.0, 500.0);

        // Raw 490 -> base dx = 500 - 490 = +10. With invert_x = true -> dx = -10.
        let (dx, _) = proc.process(490, 500);
        assert!(dx < 0, "inverted X should be negative for positive base delta");
    }

    #[test]
    fn disabled_cursor_outputs_zero() {
        let mut proc = MotionProcessor::default();
        proc.set_cursor_enabled(false);
        assert_eq!(proc.process(700, 700), (0, 0));
    }

    #[test]
    fn roll_angle_estimation_from_real_poses() {
        let mut proc = MotionProcessor::default();

        // 1. Normal (held pointing at screen): a=[2050, 2023, 1143] -> ~0°
        for _ in 0..100 {
            proc.process_with_accel(537, 537, [2050, 2023, 1143]);
        }
        assert!(proc.current_roll_deg().abs() < 2.0, "normal should be ~0° roll, got {}", proc.current_roll_deg());

        // 2. Rolled 90° right: a=[2004, 3027, 1967] -> ~+80°
        for _ in 0..100 {
            proc.process_with_accel(537, 537, [2004, 3027, 1967]);
        }
        let roll = proc.current_roll_deg();
        assert!((roll - 80.0).abs() < 5.0, "expected ~80° right, got {}", roll);

        // 3. Rolled 90° left: a=[2001, 980, 2122] -> ~-87°
        for _ in 0..100 {
            proc.process_with_accel(537, 537, [2001, 980, 2122]);
        }
        let roll = proc.current_roll_deg();
        assert!((roll - (-87.0)).abs() < 5.0, "expected ~-87° left, got {}", roll);
    }

    #[test]
    fn roll_compensation_transforms_body_axes_to_screen_up() {
        let mut settings = MotionSettings::default();
        settings.deadzone = 0.0;
        settings.smoothing = 0.0;
        settings.acceleration = 1.0;
        settings.sensitivity = 1.0;
        settings.roll_compensation = true;

        let mut proc = MotionProcessor::new(settings);
        proc.set_center(500.0, 500.0);

        // Settle at exact 90° CW: [2055, 3049, 2180]
        for _ in 0..100 {
            proc.process_with_accel(500, 500, [2055, 3049, 2180]);
        }
        assert!((proc.current_roll_deg() - 90.0).abs() < 1.0);

        // At 90° CW, moving physically UP means swinging the nose towards the remote's left edge.
        // In the remote body, that is raw_gyro_x > center (500 - 550 = -50 body yaw left).
        // With roll compensation: rot_dy = raw_dx * sin(90°) = -50 (screen UP)!
        let (dx, dy) = proc.process_with_accel(550, 500, [2055, 3049, 2180]);
        assert!(dy < 0, "screen dy should be negative (UP) when moving physically up at 90° CW, got dy={}", dy);
        assert_eq!(dx, 0, "screen dx should be 0 for pure physical up movement, got dx={}", dx);
    }

    #[test]
    fn roll_compensation_disabled_leaves_raw_body_axes() {
        let mut settings = MotionSettings::default();
        settings.deadzone = 0.0;
        settings.smoothing = 0.0;
        settings.acceleration = 1.0;
        settings.sensitivity = 1.0;
        settings.roll_compensation = false;

        let mut proc = MotionProcessor::new(settings);
        proc.set_center(500.0, 500.0);

        for _ in 0..50 {
            proc.process_with_accel(500, 500, [2055, 3049, 2180]);
        }

        // Without compensation, gyro_x delta remains strictly horizontal (dx)
        let (dx, dy) = proc.process_with_accel(550, 500, [2055, 3049, 2180]);
        assert!(dx != 0, "raw dx should be non-zero");
        assert_eq!(dy, 0, "raw dy should be 0 without roll compensation");
    }

    #[test]
    fn tremor_does_not_corrupt_roll_or_cause_vertical_drift() {
        let mut settings = MotionSettings::default();
        settings.deadzone = 0.0;
        settings.smoothing = 0.0;
        settings.acceleration = 1.0;
        settings.sensitivity = 1.0;
        settings.roll_compensation = true;

        let mut proc = MotionProcessor::new(settings);
        proc.set_center(500.0, 500.0);

        // Stabilize at normal pose: [2050, 2023, 1143]
        for _ in 0..50 {
            proc.process_with_accel(500, 500, [2050, 2023, 1143]);
        }

        // Simulate high-frequency hand tremor: alternating horizontal gyro delta +-6
        // with large synchronized lateral dynamic acceleration spikes (+-350)
        let mut cumulative_dy = 0;
        for i in 0..60 {
            let sign = if i % 2 == 0 { 1 } else { -1 };
            let gyro_x = (500 + sign * 6) as u16;
            let accel_lat = (2023 + sign * 350) as u16;
            let (_, dy) = proc.process_with_accel(gyro_x, 500, [2050, accel_lat, 1143]);
            cumulative_dy += dy;
        }

        assert_eq!(cumulative_dy, 0, "tremor must not cause cumulative vertical drift");
        assert!(proc.current_roll_deg().abs() < 1.0, "roll must stay at ~0° during tremor");
    }
}


