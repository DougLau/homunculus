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

    /// Count of points
    count: usize,

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

    /// Count of points
    count: Option<usize>,

    /// Near point limits
    near: Vec<f32>,

    /// Far point limits
    far: Vec<f32>,
}

/// Solid configuration
#[derive(Deserialize)]
pub struct Config {
    /// Vec of all rings
    ring: Vec<RingCfg>,
}

/// Solid mesh builder
struct SolidBuilder {
    builder: MeshBuilder,
    points: Vec<Point>,
}

impl RingCfg {
    /// Get point count
    fn count(&self) -> Option<usize> {
        self.count.or_else(|| {
            let count = self.near.len().max(self.far.len());
            if count > 0 {
                Some(count)
            } else {
                None
            }
        })
    }
}

impl Ring {
    /// Update ring from a configuration
    fn with_config(&mut self, cfg: RingCfg) {
        if let Some(scale) = cfg.scale {
            self.scale = scale;
        }
        if let Some(count) = cfg.count() {
            self.count = count;
            self.near.clear();
            self.near.extend_from_slice(cfg.near.as_slice());
            self.far.clear();
            self.far.extend_from_slice(cfg.far.as_slice());
        }
    }

    /// Get the near and far limits
    fn near_far(&self, i: usize) -> (f32, f32) {
        let near = self.near.get(i);
        let far = self.far.get(i);
        match (near, far) {
            (Some(near), Some(far)) => {
                if *near >= *far {
                    (*near, *near)
                } else {
                    (*near, *far)
                }
            }
            (Some(near), None) => (*near, *near),
            (None, Some(far)) => (*far, *far),
            _ => (1.0, 1.0),
        }
    }

    /// Calculate the angle of a point
    fn angle(&self, i: usize) -> f32 {
        i as f32 / self.count as f32 * PI * 2.0
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
        for i in 0..ring.count {
            let (near, _far) = ring.near_far(i);
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
        let mut points = VecDeque::new();
        for point in &self.points {
            if point.ring_number == ring0 || point.ring_number == ring1 {
                points.push_back(point);
            }
        }
        points
            .make_contiguous()
            .sort_by(|a, b| a.partial_cmp(b).unwrap());
        let ipt = points.pop_front().unwrap();
        let jpt = points.pop_front().unwrap();
        let mut ivtx = ipt.vertex;
        let mut jvtx = jpt.vertex;
        assert!(ivtx != jvtx);
        if jpt.ring_number > ipt.ring_number {
            (ivtx, jvtx) = (jvtx, ivtx);
        }
        let (avtx, bvtx) = (ivtx, jvtx);
        while let Some(pt) = points.pop_front() {
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
