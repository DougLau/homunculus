use crate::geom::Vec3;
use crate::mesh::{Face, Mesh, MeshBuilder};
use serde_derive::Deserialize;
use std::collections::VecDeque;
use std::f32::consts::PI;

/// A point on a solid surface
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Point {
    /// Ring angle (must be first for PartialOrd)
    angle: f32,

    /// Ring number
    ring_number: usize,

    /// Vertex number
    vertex: usize,
}

/// Ring on surface of solid
#[derive(Clone, Default)]
struct Ring {
    /// Ring number
    number: usize,

    /// Scale factor
    scale: f32,

    /// Near point limits
    near: Vec<f32>,

    /// Far point limits
    far: Vec<f32>,

    /// Bone vector
    bone: Vec3,
}

/// Ring configuration
#[derive(Deserialize)]
pub struct RingCfg {
    /// Ring name
    name: Option<String>,

    /// Scale factor
    scale: Option<f32>,

    /// Point limits
    points: Vec<String>,
}

/// Solid configuration
#[derive(Deserialize)]
pub struct Config {
    /// Vec of all rings
    ring: Vec<RingCfg>,
}

/// Solid mesh builder
struct SolidBuilder {
    /// Mesh builder
    builder: MeshBuilder,

    /// All points on mesh
    points: Vec<Point>,
}

impl Ring {
    /// Update ring from a configuration
    fn with_config(&mut self, cfg: RingCfg) {
        if let Some(scale) = cfg.scale {
            self.scale = scale;
        }
        if !cfg.points.is_empty() {
            (self.near, self.far) = cfg.near_far();
        }
    }

    /// Calculate the angle of a point
    fn angle(&self, i: usize) -> f32 {
        let count = self.near.len() as f32;
        i as f32 / count * PI * 2.0
    }
}

/// Parse a point count
fn parse_count(code: &str) -> usize {
    code.parse().expect("Invalid count")
}

/// Parse near/far point
fn parse_near_far(code: &str) -> (f32, f32) {
    let mut pts: Vec<f32> = code
        .split("..")
        .map(|p| p.parse::<f32>().unwrap_or(1.0))
        .collect();
    let len = pts.len();
    match len {
        1 => pts.push(pts.last().copied().unwrap()),
        2 => {
            if pts[0] > pts[1] {
                panic!("Near > far: {code}");
            }
        }
        _ => panic!("Invalid points: {code}"),
    }
    (pts[0], pts[1])
}

impl RingCfg {
    /// Get near/far points
    fn near_far(&self) -> (Vec<f32>, Vec<f32>) {
        let mut near = vec![];
        let mut far = vec![];
        let mut repeat = false;
        for code in &self.points {
            if repeat {
                let count = parse_count(code);
                let ln = near.last().copied().unwrap_or(1.0);
                let lf = far.last().copied().unwrap_or(1.0);
                for _ in 1..count {
                    near.push(ln);
                    far.push(lf);
                }
                repeat = false;
                continue;
            }
            if code == "*" {
                repeat = true;
                continue;
            }
            let (n, f) = parse_near_far(code);
            near.push(n);
            far.push(f);
        }
        (near, far)
    }
}

impl SolidBuilder {
    /// Create a new solid mesh builder
    fn new() -> SolidBuilder {
        let builder = MeshBuilder::with_capacity(128);
        let points = vec![];
        SolidBuilder { builder, points }
    }

    /// Push one point
    fn push_point(&mut self, angle: f32, ring_number: usize) {
        let vertex = self.builder.vertices();
        self.points.push(Point {
            angle,
            ring_number,
            vertex,
        });
    }

    /// Add a ring
    fn add_ring(&mut self, ring: Ring) {
        let y = ring.number as f32;
        for (i, (near, far)) in
            ring.near.iter().zip(ring.far.iter()).enumerate()
        {
            let angle = ring.angle(i);
            self.push_point(angle, ring.number);
            let dist = near * ring.scale;
            let x = dist * angle.sin();
            let z = dist * angle.cos();
            self.builder.push_vtx(Vec3([x, y, z]));
        }
    }

    /// Make a band around the solid
    fn make_band(&mut self, ring0: usize, ring1: usize) {
        let mut band = VecDeque::new();
        for point in &self.points {
            if point.ring_number == ring0 || point.ring_number == ring1 {
                band.push_back(point);
            }
        }
        band
            .make_contiguous()
            .sort_by(|a, b| a.partial_cmp(b).unwrap());
        let ipt = band.pop_front().unwrap();
        let jpt = band.pop_front().unwrap();
        let mut ivtx = ipt.vertex;
        let mut jvtx = jpt.vertex;
        assert!(ivtx != jvtx);
        if jpt.ring_number > ipt.ring_number {
            (ivtx, jvtx) = (jvtx, ivtx);
        }
        let (avtx, bvtx) = (ivtx, jvtx);
        while let Some(pt) = band.pop_front() {
            self.builder.push_face(Face::new([ivtx, jvtx, pt.vertex]));
            if pt.ring_number == ring1 {
                ivtx = pt.vertex;
            } else {
                jvtx = pt.vertex;
            }
        }
        // Connect with first vertices
        self.builder.push_face(Face::new([ivtx, jvtx, bvtx]));
        self.builder.push_face(Face::new([bvtx, avtx, ivtx]));
    }
}

impl Config {
    /// Build a mesh from the configuration
    pub fn build(self) -> Mesh {
        let mut solid = SolidBuilder::new();
        let mut ring = Ring::default();
        ring.scale = 1.0;
        for cfg in self.ring {
            ring.with_config(cfg);
            solid.add_ring(ring.clone());
            if ring.number > 0 {
                solid.make_band(ring.number - 1, ring.number);
            }
            ring.number += 1;
        }
        solid.builder.build()
    }
}
