//! 3D math, camera projection, and OBJ loading, vendored and trimmed from the
//! standalone renderer. Adapted for embedding: the loader parses from a string
//! (not a file path), tolerates `f v/vt/vn` faces, and auto-centres/scales the
//! mesh so an arbitrary CAD export fits the view.

use ndarray::{Array, Array2};

use super::colour::Colour;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tri {
    pub v1: Vec3,
    pub v2: Vec3,
    pub v3: Vec3,
}

pub fn create_x_rotation_matrix(angle_deg: f32) -> Array2<f32> {
    let mut m = Array::<f32, _>::eye(4);
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    m[[1, 1]] = cos;
    m[[1, 2]] = sin;
    m[[2, 1]] = -sin;
    m[[2, 2]] = cos;
    m
}

pub fn create_y_rotation_matrix(angle_deg: f32) -> Array2<f32> {
    let mut m = Array::<f32, _>::eye(4);
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    m[[0, 0]] = cos;
    m[[0, 2]] = sin;
    m[[2, 0]] = -sin;
    m[[2, 2]] = cos;
    m
}

pub fn create_z_rotation_matrix(angle_deg: f32) -> Array2<f32> {
    let mut m = Array::<f32, _>::eye(4);
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    m[[0, 0]] = cos;
    m[[0, 1]] = sin;
    m[[1, 0]] = -sin;
    m[[1, 1]] = cos;
    m
}

pub fn mult_vec3_mat4(v: Vec3, m: &Array2<f32>) -> Vec3 {
    let x = m[[0, 0]] * v.x + m[[1, 0]] * v.y + m[[2, 0]] * v.z + m[[3, 0]];
    let y = m[[0, 1]] * v.x + m[[1, 1]] * v.y + m[[2, 1]] * v.z + m[[3, 1]];
    let z = m[[0, 2]] * v.x + m[[1, 2]] * v.y + m[[2, 2]] * v.z + m[[3, 2]];
    let w = m[[0, 3]] * v.x + m[[1, 3]] * v.y + m[[2, 3]] * v.z + m[[3, 3]];
    if w == 0. {
        Vec3::new(x, y, z)
    } else {
        Vec3::new(x / w, y / w, z / w)
    }
}

fn normalise(v: Vec3) -> Vec3 {
    let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    Vec3::new(v.x / len, v.y / len, v.z / len)
}

pub fn normal(tri: &Tri) -> Vec3 {
    let a = tri.v2 - tri.v1;
    let b = tri.v3 - tri.v1;
    normalise(Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    ))
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

pub fn calc_tri_illum(light_dir: Vec3, tri_normal: Vec3, colour: Colour) -> Colour {
    let factor = dot(normalise(light_dir), tri_normal).max(0.05);
    colour.scale(factor)
}

/// Build a view matrix looking from `eye` at `target` with the given `up`.
pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Array2<f32> {
    let forward = normalise(target - eye);
    let up_proj = up - {
        let d = dot(up, forward);
        Vec3::new(forward.x * d, forward.y * d, forward.z * d)
    };
    let new_up = normalise(up_proj);
    let right = cross(new_up, forward);

    // point_at, then its quick inverse (transpose rotation, negate translation).
    let mut vm = Array::<f32, _>::eye(4);
    vm[[0, 0]] = right.x;
    vm[[1, 0]] = right.y;
    vm[[2, 0]] = right.z;
    vm[[0, 1]] = new_up.x;
    vm[[1, 1]] = new_up.y;
    vm[[2, 1]] = new_up.z;
    vm[[0, 2]] = forward.x;
    vm[[1, 2]] = forward.y;
    vm[[2, 2]] = forward.z;
    vm[[3, 0]] = -dot(eye, right);
    vm[[3, 1]] = -dot(eye, new_up);
    vm[[3, 2]] = -dot(eye, forward);
    vm
}

/// Perspective projection matrix for a square viewport.
pub fn projection_matrix(fov_deg: f32, near: f32, far: f32) -> Array2<f32> {
    let fov = 1. / (fov_deg / 2.).to_radians().tan();
    let q = far / (far - near);
    let mut m = Array::<f32, _>::zeros((4, 4));
    m[[0, 0]] = fov; // aspect ratio 1.0 for a square view
    m[[1, 1]] = fov;
    m[[2, 2]] = q;
    m[[2, 3]] = 1.;
    m[[3, 2]] = -q * near;
    m
}

/// Parse an OBJ from a string into triangles, centred on the mesh's bounding
/// box, uniformly scaled so its largest half-extent is 1.0, and reoriented by
/// the given base rotation (degrees, applied Z then Y then X).
///
/// Vertex texture/normal indices in `f a/b/c` faces are ignored; only the
/// position index (before the first `/`) is used. Faces with more than three
/// vertices are fan-triangulated. Per-vertex normals in the file are unused;
/// face normals are recomputed for lighting.
pub fn load_obj(obj: &str, base_rot: Vec3) -> Vec<Tri> {
    let mut verts: Vec<Vec3> = Vec::new();
    let mut faces: Vec<Vec<usize>> = Vec::new();

    for line in obj.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let coords: Vec<f32> = it.filter_map(|c| c.parse().ok()).collect();
                if coords.len() >= 3 {
                    verts.push(Vec3::new(coords[0], coords[1], coords[2]));
                }
            }
            Some("f") => {
                let idx: Vec<usize> = it
                    .filter_map(|tok| tok.split('/').next())
                    .filter_map(|s| s.parse::<usize>().ok())
                    .collect();
                if idx.len() >= 3 {
                    faces.push(idx);
                }
            }
            _ => {}
        }
    }

    // Centre on the bounding-box midpoint and scale to a unit half-extent so an
    // arbitrarily placed/sized CAD export always frames the same way.
    let (mut min, mut max) = (
        Vec3::new(f32::MAX, f32::MAX, f32::MAX),
        Vec3::new(f32::MIN, f32::MIN, f32::MIN),
    );
    for v in &verts {
        min = Vec3::new(min.x.min(v.x), min.y.min(v.y), min.z.min(v.z));
        max = Vec3::new(max.x.max(v.x), max.y.max(v.y), max.z.max(v.z));
    }
    let centre = Vec3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    );
    let half_extent = ((max.x - min.x).max(max.y - min.y).max(max.z - min.z) * 0.5).max(1e-6);

    let base = {
        let rz = create_z_rotation_matrix(base_rot.z);
        let ry = create_y_rotation_matrix(base_rot.y);
        let rx = create_x_rotation_matrix(base_rot.x);
        (rz, ry, rx)
    };
    let prep = |v: &Vec3| -> Vec3 {
        let centred = Vec3::new(
            (v.x - centre.x) / half_extent,
            (v.y - centre.y) / half_extent,
            (v.z - centre.z) / half_extent,
        );
        let r = mult_vec3_mat4(centred, &base.0);
        let r = mult_vec3_mat4(r, &base.1);
        mult_vec3_mat4(r, &base.2)
    };

    let mut tris = Vec::new();
    for face in faces {
        // Fan-triangulate: (0,1,2), (0,2,3), ...
        for i in 1..face.len() - 1 {
            let (a, b, c) = (face[0], face[i], face[i + 1]);
            if a == 0 || b == 0 || c == 0 || a > verts.len() || b > verts.len() || c > verts.len() {
                continue; // OBJ indices are 1-based; skip anything malformed.
            }
            tris.push(Tri {
                v1: prep(&verts[a - 1]),
                v2: prep(&verts[b - 1]),
                v3: prep(&verts[c - 1]),
            });
        }
    }
    tris
}
