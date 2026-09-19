//! Live 3D attitude view: renders the drone frame mesh rotated by the estimated
//! roll/pitch, into an RGBA buffer the GUI uploads as an egui texture.
//!
//! The renderer is a vendored software rasteriser (see `colour`, `raster`,
//! `geometry`), decoupled from its original `minifb` window. It is a painter's
//! -algorithm renderer with back-face culling and no depth buffer, so a concave
//! mesh can show minor sorting artefacts; that is acceptable for an orientation
//! instrument.
//!
//! Yaw is not estimated on this airframe (no magnetometer, per ADR 0022), so the
//! model is shown at a fixed heading and only rolls and pitches.
//!
//! ## Orientation tuning
//! Matching a CAD export's axes to the drone's FRD/NED conventions (ADR 0021)
//! is empirical. The `const`s below are the knobs: adjust them while watching
//! the view, one axis at a time (e.g. feed roll only, confirm the correct side
//! drops). Convention targeted after [`MODEL_BASE_ROT`]: nose -> +Z, right ->
//! +X, up -> +Y.

mod colour;
mod geometry;
mod raster;

use colour::Colour;
use geometry::{Tri, Vec3};

/// Square render resolution, in pixels. Fixed so the rasteriser can index a
/// flat framebuffer with compile-time bounds.
pub const WIDTH: usize = 480;
pub const HEIGHT: usize = 480;

/// Embedded frame mesh (exported from CAD). Parsed once at construction.
const MODEL_OBJ: &str = include_str!("../../assets/flight-frame.obj");

// --- Orientation / appearance tuning knobs -------------------------------

/// Fixed reorientation applied to the mesh at load (degrees, Z then Y then X),
/// to bring the CAD axes into the renderer's nose=+Z, right=+X, up=+Y frame.
const MODEL_BASE_ROT: Vec3 = Vec3::new(-90.0, 0.0, 0.0);

/// Sign of the applied roll and pitch. Flip to +1.0/-1.0 if a stick input tips
/// the model the wrong way.
const ROLL_SIGN: f32 = 1.0;
const PITCH_SIGN: f32 = 1.0;

/// Back-face cull sense. Flip to -1.0 if the model renders inside-out (its OBJ
/// winding is opposite to what is assumed).
const CULL_SIGN: f32 = 1.0;

/// Camera framing: distance from the origin, elevation above the horizon, and
/// azimuth around the model. A slight elevation + azimuth gives a 3/4 view in
/// which both roll and pitch read clearly. Azimuth ~180 views from behind.
const CAM_DIST: f32 = 4.2;
const CAM_ELEV_DEG: f32 = 22.0;
const CAM_AZIM_DEG: f32 = 205.0;
const FOV_DEG: f32 = 50.0;

const BACKGROUND: Colour = Colour::new(28, 30, 36);
const MODEL_ALBEDO: Colour = Colour::new(90, 160, 235);
const LIGHT_DIR: Vec3 = Vec3::new(0.4, 1.0, -0.6);

/// Owns the mesh and precomputed camera matrices, plus the framebuffers reused
/// each render to avoid per-frame allocation.
#[derive(Debug)]
pub struct AttitudeView {
    tris: Vec<Tri>,
    view: ndarray::Array2<f32>,
    proj: ndarray::Array2<f32>,
    /// `0RGB` software framebuffer.
    buffer: Vec<u32>,
    /// RGBA8 scratch handed to egui.
    rgba: Vec<u8>,
}

impl Default for AttitudeView {
    fn default() -> Self {
        Self::new()
    }
}

impl AttitudeView {
    pub fn new() -> Self {
        let tris = geometry::load_obj(MODEL_OBJ, MODEL_BASE_ROT);

        let (sin_az, cos_az) = CAM_AZIM_DEG.to_radians().sin_cos();
        let (sin_el, cos_el) = CAM_ELEV_DEG.to_radians().sin_cos();
        let eye = Vec3::new(
            CAM_DIST * cos_el * sin_az,
            CAM_DIST * sin_el,
            -CAM_DIST * cos_el * cos_az,
        );
        let view = geometry::look_at(eye, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let proj = geometry::projection_matrix(FOV_DEG, 0.1, 100.0);

        Self {
            tris,
            view,
            proj,
            buffer: vec![BACKGROUND.as_0rgb(); WIDTH * HEIGHT],
            rgba: vec![0; WIDTH * HEIGHT * 4],
        }
    }

    /// Render the mesh at the given attitude and return the RGBA8 pixels.
    pub fn render(&mut self, roll_deg: f32, pitch_deg: f32) -> &[u8] {
        self.buffer.fill(BACKGROUND.as_0rgb());

        // Attitude rotation: roll about the nose (renderer Z), pitch about the
        // right axis (renderer X). Applied roll-then-pitch.
        let rot_roll = geometry::create_z_rotation_matrix(ROLL_SIGN * roll_deg);
        let rot_pitch = geometry::create_x_rotation_matrix(PITCH_SIGN * pitch_deg);

        // (screen triangle, colour, depth) collected for the painter's sort.
        let mut drawn: Vec<(raster::Tri, u32, f32)> = Vec::with_capacity(self.tris.len());

        for tri in &self.tris {
            let world = Tri {
                v1: attitude(tri.v1, &rot_roll, &rot_pitch),
                v2: attitude(tri.v2, &rot_roll, &rot_pitch),
                v3: attitude(tri.v3, &rot_roll, &rot_pitch),
            };

            let view_tri = Tri {
                v1: geometry::mult_vec3_mat4(world.v1, &self.view),
                v2: geometry::mult_vec3_mat4(world.v2, &self.view),
                v3: geometry::mult_vec3_mat4(world.v3, &self.view),
            };

            // Cull back faces using the view-space normal.
            if geometry::normal(&view_tri).z * CULL_SIGN > 0.0 {
                continue;
            }

            let depth = (view_tri.v1.z + view_tri.v2.z + view_tri.v3.z) / 3.0;
            let colour =
                geometry::calc_tri_illum(LIGHT_DIR, geometry::normal(&world), MODEL_ALBEDO);

            let p1 = self.project(view_tri.v1);
            let p2 = self.project(view_tri.v2);
            let p3 = self.project(view_tri.v3);
            drawn.push((raster::Tri { p1, p2, p3 }, colour.as_0rgb(), depth));
        }

        // Painter's algorithm: farthest (largest view-space z) first.
        drawn.sort_by(|a, b| b.2.total_cmp(&a.2));
        for (tri, colour, _) in &drawn {
            raster::draw_filled_triangle(&mut self.buffer, tri, *colour);
        }

        for (i, px) in self.buffer.iter().enumerate() {
            self.rgba[i * 4] = ((px >> 16) & 0xff) as u8;
            self.rgba[i * 4 + 1] = ((px >> 8) & 0xff) as u8;
            self.rgba[i * 4 + 2] = (px & 0xff) as u8;
            self.rgba[i * 4 + 3] = 255;
        }
        &self.rgba
    }

    /// Project a view-space vertex to a screen-space raster point, clamping to
    /// the viewport as a safety net against off-screen coordinates.
    fn project(&self, v: Vec3) -> raster::Point {
        let ndc = geometry::mult_vec3_mat4(v, &self.proj);
        let sx = ((ndc.x + 1.0) * 0.5 * WIDTH as f32).round();
        let sy = ((ndc.y + 1.0) * 0.5 * HEIGHT as f32).round();
        raster::Point {
            x: sx.clamp(0.0, (WIDTH - 1) as f32) as u32,
            y: sy.clamp(0.0, (HEIGHT - 1) as f32) as u32,
        }
    }
}

/// Apply the attitude rotation (roll then pitch) to a model-space vertex.
fn attitude(v: Vec3, rot_roll: &ndarray::Array2<f32>, rot_pitch: &ndarray::Array2<f32>) -> Vec3 {
    let r = geometry::mult_vec3_mat4(v, rot_roll);
    geometry::mult_vec3_mat4(r, rot_pitch)
}
