use crate::mesh::{Face, Mesh, MeshBuilder, Vec3};
use std::collections::VecDeque;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Point {
    angle: f32,
    ring: usize,
    vertex: usize,
}

#[derive(Clone, Copy)]
struct Ring {
    number: usize,
    count: usize,
    up: Vec3,
    radius: f32,
}

struct SolidBuilder {
    builder: MeshBuilder,
    points: Vec<Point>,
}

impl Ring {
    fn new(count: usize) -> Ring {
        let number = 0;
        let up = Vec3([0.0, 1.0, 0.0]);
        let radius = 1.0;
        Ring { number, count, up, radius }
    }

    fn next(self, count: usize) -> Ring {
        let number = self.number + 1;
        let up = self.up;
        let radius = self.radius;
        Ring { number, count, up, radius }
    }

    fn with_radius(mut self, radius: f32) -> Ring {
        self.radius = radius;
        self
    }
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

pub fn build_solid() -> Mesh {
    let mut solid = SolidBuilder::new();
    let ring = Ring::new(8);
    solid.add_ring(ring);
    let ring = ring.next(6);
    solid.add_ring(ring);
    solid.make_band(0, 1);
    let ring = ring.next(6).with_radius(1.2);
    solid.add_ring(ring);
    solid.make_band(1, 2);
    let ring = ring.next(4).with_radius(0.75);
    solid.add_ring(ring);
    solid.make_band(2, 3);
    let ring = ring.next(3);
    solid.add_ring(ring);
    solid.make_band(3, 4);
    let ring = ring.next(1);
    solid.add_ring(ring);
    solid.make_band(4, 5);
    solid.builder.build()
}
