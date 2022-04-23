use crate::geom::Vec3;
use crate::mesh::{Face, Mesh, MeshBuilder};
use serde_derive::Deserialize;
use std::collections::VecDeque;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Point {
    angle: f32,
    ring: usize,
    vertex: usize,
}

#[derive(Clone, Copy, Default)]
struct Ring {
    number: usize,
    count: usize,
    bone: Vec3,
    radius: f32,
}

#[derive(Deserialize)]
pub struct RingCfg {
    name: Option<String>,
    count: Option<usize>,
    radius: Option<f32>,
}

#[derive(Deserialize)]
pub struct Config {
    ring: Vec<RingCfg>,
}

struct SolidBuilder {
    builder: MeshBuilder,
    points: Vec<Point>,
}

impl SolidBuilder {
    fn new() -> SolidBuilder {
        let builder = MeshBuilder::with_capacity(128);
        let points = vec![];
        SolidBuilder { builder, points }
    }

    fn push_point(&mut self, angle: f32, ring: usize) {
        let vertex = self.builder.vertices();
        self.points.push(Point {
            angle,
            ring,
            vertex,
        });
    }

    fn add_ring(&mut self, ring: Ring) {
        let y = ring.number as f32;
        for i in 0..ring.count {
            let angle = PI * 2.0 * i as f32 / ring.count as f32;
            self.push_point(angle, ring.number);
            let x = ring.radius * angle.sin();
            let z = ring.radius * angle.cos();
            self.builder.push_vtx(Vec3([x, y, z]));
        }
    }

    fn make_band(&mut self, ring0: usize, ring1: usize) {
        let mut points = VecDeque::new();
        for point in &self.points {
            if point.ring == ring0 || point.ring == ring1 {
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
        if jpt.ring > ipt.ring {
            (ivtx, jvtx) = (jvtx, ivtx);
        }
        let (avtx, bvtx) = (ivtx, jvtx);
        while let Some(pt) = points.pop_front() {
            self.builder.push_face(Face::new([ivtx, jvtx, pt.vertex]));
            if pt.ring == ring1 {
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
    pub fn build(self) -> Mesh {
        let mut solid = SolidBuilder::new();
        let mut ring = Ring::default();
        ring.radius = 1.0;
        for r in self.ring {
            if let Some(count) = r.count {
                ring.count = count;
            }
            if let Some(radius) = r.radius {
                ring.radius = radius;
            }
            solid.add_ring(ring);
            if ring.number > 0 {
                solid.make_band(ring.number - 1, ring.number);
            }
            ring.number += 1;
        }
        solid.builder.build()
    }
}
