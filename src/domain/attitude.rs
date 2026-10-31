//! 3D Attitude Estimator using Direction Cosine Matrix (DCM) sensor fusion.
//!
//! Fuses 2-axis high-speed gyro angular rates (yaw, pitch) with 3-axis accelerometer
//! gravity vector to maintain a continuous, singularity-free 3D orientation matrix.
//!
//! Unlike naive Euler angle implementations, this maintains a full SO(3) rotation
//! matrix, preventing gimbal lock, roll flips, and perspective inversions at ±90° or ±180°.

use crate::domain::gravity::AccelCalibration;

/// 3x3 Matrix representing orientation in SO(3).
pub type Mat3 = [[f32; 3]; 3];

pub const MAT3_IDENTITY: Mat3 = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];

/// Multiplies two 3x3 matrices: C = A * B.
pub fn mat3_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = [[0.0f32; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            out[r][c] = a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c];
        }
    }
    out
}

/// Transposes a 3x3 matrix: out = A^T.
pub fn mat3_transpose(a: &Mat3) -> Mat3 {
    [
        [a[0][0], a[1][0], a[2][0]],
        [a[0][1], a[1][1], a[2][1]],
        [a[0][2], a[1][2], a[2][2]],
    ]
}

/// Orthonormalizes a 3x3 matrix via Gram-Schmidt to maintain pure SO(3) rotation.
pub fn mat3_orthonormalize(m: &Mat3) -> Mat3 {
    let mut x = [m[0][0], m[1][0], m[2][0]];
    let mut y = [m[0][1], m[1][1], m[2][1]];

    // Normalize X
    let len_x = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt().max(1e-6);
    x[0] /= len_x;
    x[1] /= len_x;
    x[2] /= len_x;

    // Project Y onto X and subtract
    let dot_xy = y[0] * x[0] + y[1] * x[1] + y[2] * x[2];
    y[0] -= dot_xy * x[0];
    y[1] -= dot_xy * x[1];
    y[2] -= dot_xy * x[2];

    let len_y = (y[0] * y[0] + y[1] * y[1] + y[2] * y[2]).sqrt().max(1e-6);
    y[0] /= len_y;
    y[1] /= len_y;
    y[2] /= len_y;

    // Z = X cross Y
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];

    [
        [x[0], y[0], z[0]],
        [x[1], y[1], z[1]],
        [x[2], y[2], z[2]],
    ]
}

/// Continuous 3D Attitude Estimator.
#[derive(Clone, Copy, Debug)]
pub struct AttitudeEstimator {
    /// 3x3 Direction Cosine Matrix mapping body coordinates to world coordinates.
    /// Columns are: [right_axis, nose_axis, up_axis] in world frame.
    rot: Mat3,
    /// Reference zero matrix set when user clicks recenter.
    tare: Mat3,
    /// Accelerometer calibration
    calibration: AccelCalibration,
    /// Gravity correction gain (Kp)
    kp: f32,
    /// Calibrated gyro scale in radians/count
    gyro_scale: f32,
}

impl AttitudeEstimator {
    pub fn new(calibration: AccelCalibration) -> Self {
        Self {
            rot: MAT3_IDENTITY,
            tare: MAT3_IDENTITY,
            calibration,
            kp: 3.0,
            // 0.0250 deg/count per frame * 100 frames/s = 2.5 deg/s per count = ~0.043633 rad/s/count
            gyro_scale: 0.043633,
        }
    }

    /// Resets orientation to neutral identity matrix.
    pub fn reset(&mut self) {
        self.rot = MAT3_IDENTITY;
        self.tare = MAT3_IDENTITY;
    }

    /// Sets the current physical resting pose as the new reference ZERO orientation.
    pub fn recenter(&mut self) {
        self.tare = self.rot;
    }

    /// Updates attitude filter with one sensor frame.
    /// - `gyro_x`: raw 16-bit gyro Yaw rate
    /// - `gyro_y`: raw 16-bit gyro Pitch rate
    /// - `center_x`, `center_y`: calibrated gyro center points
    /// - `raw_accel`: raw accelerometer ADC counts [a0 (nose), a1 (right), a2 (up)]
    /// - `dt`: delta time in seconds (~0.010s for 100Hz)
    pub fn update(
        &mut self,
        gyro_x: u16,
        gyro_y: u16,
        center_x: f32,
        center_y: f32,
        raw_accel: [u16; 3],
        dt: f32,
    ) {
        // Accelerometer readings converted to g in mesh body frame:
        // Remote hardware body:
        // accel[0] = longitudinal nose (+Y in mesh)
        // accel[1] = lateral right (+X in mesh)
        // accel[2] = normal buttons (+Z in mesh)
        let [g_nose, g_right, g_buttons] = self.calibration.to_g(raw_accel);
        let g_meas = [g_right, g_nose, g_buttons];

        // Expected gravity in body frame: R^T * [0, 0, -1] = -third row of rot
        let g_exp = [-self.rot[2][0], -self.rot[2][1], -self.rot[2][2]];

        let acc_mag = (g_meas[0] * g_meas[0] + g_meas[1] * g_meas[1] + g_meas[2] * g_meas[2]).sqrt();
        // Trust factor: 1.0 at rest (1g), drops to 0 during fast hand flicks/acceleration to prevent shaking
        let trust = (1.0 - (acc_mag - 1.0).abs() / 0.25).clamp(0.0, 1.0);

        let err = if acc_mag > 0.5 && acc_mag < 1.5 {
            let u = [g_meas[0] / acc_mag, g_meas[1] / acc_mag, g_meas[2] / acc_mag];
            // Cross product: u x g_exp
            let mut e = [
                u[1] * g_exp[2] - u[2] * g_exp[1],
                u[2] * g_exp[0] - u[0] * g_exp[2],
                u[0] * g_exp[1] - u[1] * g_exp[0],
            ];
            // Handle antiparallel saddle point when completely upside down (dot ≈ -1, cross product ≈ 0)
            let dot = u[0] * g_exp[0] + u[1] * g_exp[1] + u[2] * g_exp[2];
            if dot < -0.95 && (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]) < 1e-3 {
                e[1] = 1.0; // kick around roll axis to break symmetry
            }
            e
        } else {
            [0.0, 0.0, 0.0]
        };

        // Gyro rates around body axes:
        // Turning RIGHT: gyro_x < center_x -> raw_yaw < 0 -> rotates nose to +X (right)
        let raw_yaw = gyro_x as f32 - center_x;
        // Pitching UP: gyro_y > center_y -> raw_pitch > 0 -> rotates nose to +Z (up)
        let raw_pitch = gyro_y as f32 - center_y;

        let w_yaw = if raw_yaw.abs() > 1.8 {
            raw_yaw * self.gyro_scale
        } else {
            0.0
        };

        let w_pitch = if raw_pitch.abs() > 1.8 {
            raw_pitch * self.gyro_scale
        } else {
            0.0
        };

        // Total body angular velocity vector:
        // omega_x = pitch rate around X + pitch correction
        // omega_y = roll correction around Y (adaptive gain for instant wrist tracking)
        // omega_z = yaw rate around Z + yaw correction
        let k_roll = 15.0 + 20.0 * err[1].abs();
        let omega = [
            w_pitch + self.kp * trust * err[0],
            k_roll * trust * err[1],
            w_yaw + self.kp * trust * err[2],
        ];

        let angle = (omega[0] * omega[0] + omega[1] * omega[1] + omega[2] * omega[2]).sqrt() * dt;
        if angle > 1e-6 {
            let inv_len = 1.0 / (omega[0] * omega[0] + omega[1] * omega[1] + omega[2] * omega[2]).sqrt();
            let ax = [omega[0] * inv_len, omega[1] * inv_len, omega[2] * inv_len];

            let (sin_a, cos_a) = angle.sin_cos();
            let c1 = 1.0 - cos_a;

            // Rodrigues rotation matrix dR:
            let dr = [
                [
                    cos_a + ax[0] * ax[0] * c1,
                    ax[0] * ax[1] * c1 - ax[2] * sin_a,
                    ax[0] * ax[2] * c1 + ax[1] * sin_a,
                ],
                [
                    ax[1] * ax[0] * c1 + ax[2] * sin_a,
                    cos_a + ax[1] * ax[1] * c1,
                    ax[1] * ax[2] * c1 - ax[0] * sin_a,
                ],
                [
                    ax[2] * ax[0] * c1 - ax[1] * sin_a,
                    ax[2] * ax[1] * c1 + ax[0] * sin_a,
                    cos_a + ax[2] * ax[2] * c1,
                ],
            ];

            let new_rot = mat3_mul(&self.rot, &dr);
            self.rot = mat3_orthonormalize(&new_rot);
        }
    }

    /// Returns the orientation matrix relative to the calibrated tare zero pose:
    /// R_rel = tare^T * rot.
    pub fn relative_matrix(&self) -> Mat3 {
        let tare_t = mat3_transpose(&self.tare);
        mat3_mul(&tare_t, &self.rot)
    }

    /// Returns (yaw_deg, pitch_deg, roll_deg) extracted from the relative orientation matrix.
    pub fn angles_deg(&self) -> (f32, f32, f32) {
        let r = self.relative_matrix();
        let right = [r[0][0], r[1][0], r[2][0]];
        let nose = [r[0][1], r[1][1], r[2][1]];
        let up = [r[0][2], r[1][2], r[2][2]];

        let horiz = (nose[0] * nose[0] + nose[1] * nose[1]).sqrt();
        let pitch = nose[2].atan2(horiz).to_degrees();
        let yaw = nose[0].atan2(nose[1]).to_degrees();
        let roll = (-right[2]).atan2(up[2]).to_degrees();

        (yaw, pitch, roll)
    }
}

impl Default for AttitudeEstimator {
    fn default() -> Self {
        Self::new(AccelCalibration::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_and_recenter() {
        let mut est = AttitudeEstimator::default();
        let (y, p, r) = est.angles_deg();
        assert!(y.abs() < 1e-4);
        assert!(p.abs() < 1e-4);
        assert!(r.abs() < 1e-4);

        est.recenter();
        let (y2, p2, r2) = est.angles_deg();
        assert!(y2.abs() < 1e-4);
        assert!(p2.abs() < 1e-4);
        assert!(r2.abs() < 1e-4);
    }

    #[test]
    fn test_mat3_orthonormalize() {
        let noisy = [
            [1.02, 0.01, -0.02],
            [-0.01, 0.99, 0.03],
            [0.02, -0.03, 1.01],
        ];
        let ortho = mat3_orthonormalize(&noisy);
        let len_x = (ortho[0][0] * ortho[0][0] + ortho[1][0] * ortho[1][0] + ortho[2][0] * ortho[2][0]).sqrt();
        let len_y = (ortho[0][1] * ortho[0][1] + ortho[1][1] * ortho[1][1] + ortho[2][1] * ortho[2][1]).sqrt();
        let len_z = (ortho[0][2] * ortho[0][2] + ortho[1][2] * ortho[1][2] + ortho[2][2] * ortho[2][2]).sqrt();
        assert!((len_x - 1.0).abs() < 1e-4);
        assert!((len_y - 1.0).abs() < 1e-4);
        assert!((len_z - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_upside_down_does_not_flip_pitch() {
        let mut est = AttitudeEstimator::default();
        // Upside down raw accel: a0 = 2055, a1 = 2012, a2 = 3203 (+1g)
        for _ in 0..100 {
            est.update(537, 537, 537.0, 537.0, [2055, 2012, 3203], 0.010);
        }
        let (_y, p, r) = est.angles_deg();
        assert!(p.abs() < 10.0, "Pitch should remain near 0° when upside down, got {p}");
        assert!((r.abs() - 180.0).abs() < 10.0, "Roll should be near 180°, got {r}");
    }

    #[test]
    fn test_turning_while_rolled_90_degrees() {
        let mut est = AttitudeEstimator::default();
        // Settle at 90° roll (a1 = 3049)
        for _ in 0..80 {
            est.update(537, 537, 537.0, 537.0, [2055, 3049, 2180], 0.010);
        }
        let (y0, _p0, r0) = est.angles_deg();
        assert!((r0.abs() - 90.0).abs() < 15.0, "Should be rolled ~90°, got {r0}");

        // Now turn left/right in room: since rolled 90°, rotation in room is detected on gyro_y (lateral body axis)
        for _ in 0..20 {
            est.update(537, 437, 537.0, 537.0, [2055, 3049, 2180], 0.010);
        }
        let (y1, _p1, _r1) = est.angles_deg();
        assert!((y1 - y0).abs() > 5.0, "Yaw should change when turning while rolled 90°, got y0={y0}, y1={y1}");
    }

    #[test]
    fn test_anti_shake_trust_factor_rejects_spikes() {
        let mut est = AttitudeEstimator::default();
        // Stabilize at rest
        for _ in 0..50 {
            est.update(537, 537, 537.0, 537.0, [2055, 2012, 1143], 0.010);
        }
        let (_, p0, r0) = est.angles_deg();

        // Inject severe acceleration spikes (2.5g linear flick) with gyro calm
        for _ in 0..10 {
            est.update(537, 537, 537.0, 537.0, [3500, 3500, 3500], 0.010);
        }
        let (_, p1, r1) = est.angles_deg();
        assert!((p1 - p0).abs() < 2.0, "Pitch should not jump during violent acceleration spike");
        assert!((r1 - r0).abs() < 2.0, "Roll should not jump during violent acceleration spike");
    }
}
