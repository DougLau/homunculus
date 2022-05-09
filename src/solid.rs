use crate::geom::Vec3;
use crate::mesh::{Face, Mesh, MeshBuilder};
use serde_derive::Deserialize;
use std::collections::VecDeque;
use std::f32::consts::PI;

/// A point on a solid surface
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Point {
    /// Ring number
    ring_number: usize,

    /// Ring angle
    angle: f32,

    /// Vertex number
    vertex: usize,
}

/// Ring on surface of solid
#[derive(Clone, Default)]
struct Ring {
    /// Ring number
    number: usize,

    /// Radius of ring
    radius: f32,

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

    /// Ring radius
    radius: Option<f32>,

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

impl Ring {
    /// Update ring from a configuration
    fn with_config(&mut self, r: RingCfg) {
        if let Some(radius) = r.radius {
            self.radius = radius;
        }
        match r.count {
            Some(count) => {
                self.count = count;
                self.near.clear();
                self.far.clear();
            }
            None => {
                if !r.near.is_empty() || !r.far.is_empty() {
                    self.count = r.near.len().max(r.far.len());
                }
            }
        }
        let count = self.count;
        if !r.near.is_empty() {
            self.near.clear();
            self.near.extend_from_slice(r.near.as_slice());
        }
        if !r.far.is_empty() {
            self.far.clear();
            self.far.extend_from_slice(r.far.as_slice());
        }
        for i in 0..count {
            let near = self.near.get(i);
            let far = self.far.get(i);
            match (near, far) {
                (Some(near), Some(far)) => {
                    if near >= far {
                        self.far[i] = *near;
                    }
                }
                (Some(near), None) => self.far.push(*near),
                (None, Some(far)) => self.near.push(*far),
                _ => {
                    self.near.push(1.0);
                    self.far.push(1.0);
                }
            }
        }
        self.near.resize(count, 1.0);
        self.far.resize(count, 1.0);
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
            ring_number,
            angle,
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
            let dist = near;
            let x = dist * ring.radius * angle.sin();
            let z = dist * ring.radius * angle.cos();
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
        ring.radius = 1.0;
        for r in self.ring {
            ring.with_config(r);
            solid.add_ring(ring.clone());
            if ring.number > 0 {
                solid.make_band(ring.number - 1, ring.number);
            }
            ring.number += 1;
        }
        solid.builder.build()
    }
}
