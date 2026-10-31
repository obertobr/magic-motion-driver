//! 3D Mesh representation, Wavefront .obj loader, and lightweight software renderer.
//!
//! Provides real 3D rendering for egui with backface culling, Lambertian diffuse
//! lighting, and depth-sorted painter's algorithm without requiring heavyweight
//! external 3D engines or GPU runtimes.

use std::fs;
use std::path::Path;
use eframe::egui::{self, Color32, Painter, Pos2, Stroke};

#[derive(Clone, Copy, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 1e-5 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Self::new(0.0, 0.0, 1.0)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub v0: usize,
    pub v1: usize,
    pub v2: usize,
    pub base_color: Color32,
}

#[derive(Clone, Debug)]
pub struct Mesh3D {
    pub name: String,
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<Triangle>,
}

impl Mesh3D {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vertices: Vec::new(),
            triangles: Vec::new(),
        }
    }

    /// Loads a Wavefront .obj file from the filesystem.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Falha ao ler arquivo .obj: {e}"))?;
        let name = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Custom Model");
        Self::from_obj_str(&content, name)
    }

    /// Parses a Wavefront .obj string. Supports standard vertex ('v') and face ('f') formats.
    pub fn from_obj_str(content: &str, model_name: &str) -> Result<Self, String> {
        let mut mesh = Self::new(model_name);

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("v") => {
                    let x: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let y: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let z: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    mesh.vertices.push(Vec3::new(x, y, z));
                }
                Some("f") => {
                    let mut face_indices = Vec::new();
                    for part in parts {
                        // Handle v, v/vt, or v/vt/vn formats
                        let v_str = part.split('/').next().unwrap_or("");
                        if let Ok(idx) = v_str.parse::<i32>() {
                            let actual_idx = if idx > 0 {
                                (idx - 1) as usize
                            } else {
                                (mesh.vertices.len() as i32 + idx) as usize
                            };
                            face_indices.push(actual_idx);
                        }
                    }

                    // Triangulate polygons (fan triangulation)
                    if face_indices.len() >= 3 {
                        for i in 1..face_indices.len() - 1 {
                            mesh.triangles.push(Triangle {
                                v0: face_indices[0],
                                v1: face_indices[i],
                                v2: face_indices[i + 1],
                                base_color: Color32::from_rgb(62, 75, 96),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        if mesh.vertices.is_empty() || mesh.triangles.is_empty() {
            return Err("Arquivo .obj inválido ou sem geometria.".to_string());
        }

        mesh.auto_scale_and_center(130.0);
        Ok(mesh)
    }

    /// Normalizes and centers the mesh to fit nicely within standard remote dimensions (~130 units long).
    pub fn auto_scale_and_center(&mut self, target_length: f32) {
        if self.vertices.is_empty() {
            return;
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        for v in &self.vertices {
            min_x = min_x.min(v.x);
            max_x = max_x.max(v.x);
            min_y = min_y.min(v.y);
            max_y = max_y.max(v.y);
            min_z = min_z.min(v.z);
            max_z = max_z.max(v.z);
        }

        let center_x = (min_x + max_x) * 0.5;
        let center_y = (min_y + max_y) * 0.5;
        let center_z = (min_z + max_z) * 0.5;

        let span_x = (max_x - min_x).max(1e-3);
        let span_y = (max_y - min_y).max(1e-3);
        let span_z = (max_z - min_z).max(1e-3);

        // Remote models typically have the largest span along the longitudinal axis
        let max_span = span_x.max(span_y).max(span_z);
        let scale_factor = target_length / max_span;

        for v in &mut self.vertices {
            v.x = (v.x - center_x) * scale_factor;
            v.y = (v.y - center_y) * scale_factor;
            v.z = (v.z - center_z) * scale_factor;
        }
    }

    /// Tries to load `remote.obj` from current directory, `assets/remote.obj`,
    /// or falls back to the built-in smooth curved LG Magic Remote mesh.
    pub fn load_or_default() -> Self {
        // Load custom model only if remote.obj is explicitly provided
        let candidates = [
            Path::new("remote.obj"),
        ];

        for path in &candidates {
            if path.exists() {
                if let Ok(mesh) = Self::load_from_file(path) {
                    return mesh;
                }
            }
        }

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let exe_obj = parent.join("remote.obj");
                if exe_obj.exists() {
                    if let Ok(mesh) = Self::load_from_file(&exe_obj) {
                        return mesh;
                    }
                }
            }
        }

        Self::create_curved_lg_remote()
    }

    /// Generates a smooth, curved 3D model of the LG Magic Remote (AN-MR200).
    /// Features continuous aerodynamic contours, rounded bullet nose with IR tip,
    /// contoured waist grip, recessed wheel platform, and navigation wheel.
    pub fn create_curved_lg_remote() -> Self {
        let mut mesh = Self::new("LG Magic Remote (Nativo 3D)");

        // 9 Cross-sections along the Y longitudinal axis (from tail to nose)
        // (y, half_width, top_z, bottom_z)
        let sections: [(f32, f32, f32, f32); 9] = [
            (-65.0, 11.0, 5.0, -5.0),   // 0: Tail cap
            (-50.0, 14.5, 7.5, -7.5),   // 1: Lower grip
            (-25.0, 16.5, 8.8, -8.8),   // 2: Waist curve
            (0.0,   18.0, 9.5, -9.0),   // 3: Palm center
            (22.0,  17.5, 9.2, -8.5),   // 4: Scroll wheel zone
            (42.0,  15.5, 8.2, -7.5),   // 5: D-pad / Top keypad
            (56.0,  13.0, 6.8, -6.5),   // 6: Upper nose taper
            (66.0,  9.0,  5.0, -4.5),   // 7: Front lens bezel
            (72.0,  5.0,  3.0, -2.5),   // 8: IR transmitter tip
        ];

        const SECTOR_COUNT: usize = 10;
        let mut ring_offsets = Vec::new();

        // Generate rings of vertices around each section
        for &(y, hw, tz, bz) in &sections {
            let start_idx = mesh.vertices.len();
            ring_offsets.push(start_idx);

            for s in 0..SECTOR_COUNT {
                let angle = (s as f32 / SECTOR_COUNT as f32) * std::f32::consts::TAU;
                let (sin_a, cos_a) = angle.sin_cos();

                // Elliptical cross section with asymmetry between top and bottom
                let x = cos_a * hw;
                let z = if sin_a >= 0.0 {
                    sin_a * tz
                } else {
                    sin_a * (-bz)
                };

                mesh.vertices.push(Vec3::new(x, y, z));
            }
        }

        // Bridge adjacent rings with quads (split into 2 triangles)
        for r in 0..sections.len() - 1 {
            let r0 = ring_offsets[r];
            let r1 = ring_offsets[r + 1];
            let is_nose = r >= 6;

            for s in 0..SECTOR_COUNT {
                let s_next = (s + 1) % SECTOR_COUNT;
                let i00 = r0 + s;
                let i01 = r0 + s_next;
                let i10 = r1 + s;
                let i11 = r1 + s_next;

                // Color grading by section and orientation (top vs bottom)
                let is_top = s <= SECTOR_COUNT / 2;
                let base_color = if is_nose {
                    Color32::from_rgb(190, 35, 55) // Crimson IR nose
                } else if is_top {
                    Color32::from_rgb(58, 70, 90)  // Slate top shell
                } else {
                    Color32::from_rgb(26, 32, 42)  // Charcoal ergonomic base
                };

                // Triangulation with outward-facing counter-clockwise normal
                mesh.triangles.push(Triangle { v0: i00, v1: i10, v2: i11, base_color });
                mesh.triangles.push(Triangle { v0: i00, v1: i11, v2: i01, base_color });
            }
        }

        // Close rear tail cap
        let tail_center_idx = mesh.vertices.len();
        mesh.vertices.push(Vec3::new(0.0, -66.5, 0.0));
        let r0 = ring_offsets[0];
        for s in 0..SECTOR_COUNT {
            let s_next = (s + 1) % SECTOR_COUNT;
            mesh.triangles.push(Triangle {
                v0: tail_center_idx,
                v1: r0 + s,
                v2: r0 + s_next,
                base_color: Color32::from_rgb(24, 28, 36),
            });
        }

        // Close front IR sensor tip
        let nose_center_idx = mesh.vertices.len();
        mesh.vertices.push(Vec3::new(0.0, 73.5, 0.0));
        let r_last = ring_offsets[sections.len() - 1];
        for s in 0..SECTOR_COUNT {
            let s_next = (s + 1) % SECTOR_COUNT;
            mesh.triangles.push(Triangle {
                v0: nose_center_idx,
                v1: r_last + s_next,
                v2: r_last + s,
                base_color: Color32::from_rgb(230, 45, 70),
            });
        }

        // Add 3D Navigation Wheel (Cylinder distinctly elevated above top surface at Y = 22.0)
        let wheel_center_y = 22.0f32;
        let wheel_radius = 5.2f32;
        let wheel_base_z = 9.8f32;
        let wheel_top_z = 13.0f32;
        let wheel_ring_base = mesh.vertices.len();

        const WHEEL_SECTORS: usize = 8;
        for s in 0..WHEEL_SECTORS {
            let angle = (s as f32 / WHEEL_SECTORS as f32) * std::f32::consts::TAU;
            let (sin_a, cos_a) = angle.sin_cos();
            mesh.vertices.push(Vec3::new(cos_a * wheel_radius, wheel_center_y + sin_a * (wheel_radius * 1.3), wheel_base_z));
        }
        let wheel_ring_top = mesh.vertices.len();
        for s in 0..WHEEL_SECTORS {
            let angle = (s as f32 / WHEEL_SECTORS as f32) * std::f32::consts::TAU;
            let (sin_a, cos_a) = angle.sin_cos();
            mesh.vertices.push(Vec3::new(cos_a * (wheel_radius * 0.85), wheel_center_y + sin_a * (wheel_radius * 1.1), wheel_top_z));
        }
        let wheel_hub_idx = mesh.vertices.len();
        mesh.vertices.push(Vec3::new(0.0, wheel_center_y, wheel_top_z + 0.4));

        for s in 0..WHEEL_SECTORS {
            let s_next = (s + 1) % WHEEL_SECTORS;
            // Sides of wheel
            mesh.triangles.push(Triangle {
                v0: wheel_ring_base + s,
                v1: wheel_ring_top + s_next,
                v2: wheel_ring_top + s,
                base_color: Color32::from_rgb(20, 24, 32),
            });
            mesh.triangles.push(Triangle {
                v0: wheel_ring_base + s,
                v1: wheel_ring_base + s_next,
                v2: wheel_ring_top + s_next,
                base_color: Color32::from_rgb(20, 24, 32),
            });
            // Top wheel cap
            mesh.triangles.push(Triangle {
                v0: wheel_hub_idx,
                v1: wheel_ring_top + s,
                v2: wheel_ring_top + s_next,
                base_color: Color32::from_rgb(0, 215, 255), // Cyan scroll wheel highlight
            });
        }

        // Add 3D Power Button on upper top surface (Y = 48.0)
        let power_y = 48.0f32;
        let power_hub = mesh.vertices.len();
        mesh.vertices.push(Vec3::new(0.0, power_y, 9.4));
        let power_ring_start = mesh.vertices.len();
        const POWER_SECTORS: usize = 6;
        for s in 0..POWER_SECTORS {
            let angle = (s as f32 / POWER_SECTORS as f32) * std::f32::consts::TAU;
            let (sin_a, cos_a) = angle.sin_cos();
            mesh.vertices.push(Vec3::new(cos_a * 3.5, power_y + sin_a * 3.5, 8.8));
        }
        for s in 0..POWER_SECTORS {
            let s_next = (s + 1) % POWER_SECTORS;
            mesh.triangles.push(Triangle {
                v0: power_hub,
                v1: power_ring_start + s,
                v2: power_ring_start + s_next,
                base_color: Color32::from_rgb(255, 60, 60), // Red power button
            });
        }

        mesh
    }

    /// Renders the 3D mesh onto the egui Painter using full 3D matrix orientation,
    /// normal 3D perspective looking slightly downward from behind, 3D ground plane reference grid,
    /// Lambertian diffuse lighting, backface culling, and depth-sorted painter's algorithm.
    pub fn render(
        &self,
        painter: &Painter,
        center: Pos2,
        rot: &crate::domain::attitude::Mat3,
        view_scale: f32,
    ) {
        if self.vertices.is_empty() || self.triangles.is_empty() {
            return;
        }

        // Fixed camera placed behind and above the remote, tilted slightly downward (~14°)
        // provides natural 3D depth, viewing the remote top surface and buttons from above.
        let cam_elevation = 14.0f32.to_radians();
        let (sin_e, cos_e) = cam_elevation.sin_cos();
        let cam_dist = 280.0f32;

        let project_cam = |x3: f32, y3: f32, z3: f32| -> (Pos2, Vec3) {
            let x_cam = x3;
            let depth = (cam_dist + y3 * cos_e - z3 * sin_e).max(40.0);
            let y_cam = y3 * sin_e + z3 * cos_e;

            let proj_scale = (cam_dist / depth) * view_scale;
            let px = center.x + x_cam * proj_scale;
            let py = center.y - y_cam * proj_scale;
            (Pos2::new(px, py), Vec3::new(x_cam, depth, y_cam))
        };

        // Transform all remote vertices using full 3D rotation matrix (no Euler gimbal lock / flips)
        let mut screen_points: Vec<Pos2> = Vec::with_capacity(self.vertices.len());
        let mut cam_coords: Vec<Vec3> = Vec::with_capacity(self.vertices.len());

        for v in &self.vertices {
            let x3 = rot[0][0] * v.x + rot[0][1] * v.y + rot[0][2] * v.z;
            let y3 = rot[1][0] * v.x + rot[1][1] * v.y + rot[1][2] * v.z;
            let z3 = rot[2][0] * v.x + rot[2][1] * v.y + rot[2][2] * v.z;

            let (p, c) = project_cam(x3, y3, z3);
            screen_points.push(p);
            cam_coords.push(c);
        }

        // Directional Key Light from upper-front-left in camera space
        let light = Vec3::new(-0.35, 0.65, 0.65).normalized();

        struct RenderFace {
            depth: f32,
            pts: [Pos2; 3],
            shaded_color: Color32,
        }

        let mut faces_to_draw: Vec<RenderFace> = Vec::with_capacity(self.triangles.len());

        for tri in &self.triangles {
            let c0 = cam_coords[tri.v0];
            let c1 = cam_coords[tri.v1];
            let c2 = cam_coords[tri.v2];

            // True 3D Outward Normal in Camera Space:
            let e1 = c1.sub(c0);
            let e2 = c2.sub(c0);
            let normal = e1.cross(e2).normalized();

            // Perspective view ray from camera optical origin (0, 0, 0) to vertex c0
            let view_ray = Vec3::new(c0.x, c0.y, c0.z);
            if normal.dot(view_ray) >= 0.0 {
                // Backface pointing away from camera optical center - cull
                continue;
            }

            let avg_depth = (c0.y + c1.y + c2.y) * 0.3333333;

            // Lambertian directional lighting (smooth diffuse shading)
            let dot = (-normal.dot(light)).clamp(0.0, 1.0);
            let shade_factor = (0.38 + 0.62 * dot).clamp(0.2, 1.0);

            let r = ((tri.base_color.r() as f32) * shade_factor) as u8;
            let g = ((tri.base_color.g() as f32) * shade_factor) as u8;
            let b = ((tri.base_color.b() as f32) * shade_factor) as u8;

            let p0 = screen_points[tri.v0];
            let p1 = screen_points[tri.v1];
            let p2 = screen_points[tri.v2];

            faces_to_draw.push(RenderFace {
                depth: avg_depth,
                pts: [p0, p1, p2],
                shaded_color: Color32::from_rgb(r, g, b),
            });
        }

        // Painter's algorithm: sort farthest (largest positive Y depth) to closest
        faces_to_draw.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));

        // Draw solid smooth polygons with Stroke::NONE to eliminate line flickering
        for face in faces_to_draw {
            painter.add(egui::Shape::convex_polygon(
                face.pts.to_vec(),
                face.shaded_color,
                Stroke::NONE,
            ));
        }
    }

    /// Exports the current mesh to Wavefront .obj format string.
    pub fn to_obj_string(&self) -> String {
        let mut out = String::with_capacity(self.vertices.len() * 32 + self.triangles.len() * 24);
        out.push_str(&format!("# {}\n", self.name));
        for v in &self.vertices {
            out.push_str(&format!("v {:.4} {:.4} {:.4}\n", v.x, v.y, v.z));
        }
        for tri in &self.triangles {
            // Wavefront 1-based indexing
            out.push_str(&format!("f {} {} {}\n", tri.v0 + 1, tri.v1 + 1, tri.v2 + 1));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_built_in_curved_remote_validity() {
        let remote = Mesh3D::create_curved_lg_remote();
        assert!(!remote.vertices.is_empty());
        assert!(!remote.triangles.is_empty());
        for v in &remote.vertices {
            assert!(!v.x.is_nan() && !v.y.is_nan() && !v.z.is_nan());
        }
        for tri in &remote.triangles {
            assert!(tri.v0 < remote.vertices.len());
            assert!(tri.v1 < remote.vertices.len());
            assert!(tri.v2 < remote.vertices.len());
        }
    }

    #[test]
    fn test_obj_roundtrip() {
        let original = Mesh3D::create_curved_lg_remote();
        let obj_str = original.to_obj_string();
        let loaded = Mesh3D::from_obj_str(&obj_str, "Roundtrip").expect("failed to load generated obj");
        assert_eq!(original.vertices.len(), loaded.vertices.len());
        assert_eq!(original.triangles.len(), loaded.triangles.len());
    }

    #[test]
    fn test_parse_obj_quads_and_comments() {
        let obj = "
            # Comment line
            v -10.0 -10.0 0.0
            v 10.0 -10.0 0.0
            v 10.0 10.0 0.0
            v -10.0 10.0 0.0
            f 1 2 3 4
        ";
        let mesh = Mesh3D::from_obj_str(obj, "Quad").unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.triangles.len(), 2);
    }
}
