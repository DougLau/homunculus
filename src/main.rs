pub mod gltf;
pub mod mesh;

use mesh::{Face, Mesh, MeshBuilder, Vec3};
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

fn build_solid() -> Mesh {
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

fn build_cube() -> Mesh {
    let mut builder = MeshBuilder::with_capacity(16);
    builder.push_vtx(Vec3([-0.5, -0.5, 0.5])); // 0 left bottom front
    builder.push_vtx(Vec3([-0.5, 0.5, 0.5])); // 1 left top front
    builder.push_vtx(Vec3([0.5, -0.5, 0.5])); // 2 right bottom front
    builder.push_vtx(Vec3([0.5, 0.5, 0.5])); // 3 right top front
    builder.push_vtx(Vec3([-0.5, -0.5, -0.5])); // 4 left bottom back
    builder.push_vtx(Vec3([-0.5, 0.5, -0.5])); // 5 left top back
    builder.push_vtx(Vec3([0.5, -0.5, -0.5])); // 6 right bottom back
    builder.push_vtx(Vec3([0.5, 0.5, -0.5])); // 7 right top back

    // front
    builder.push_face(Face::new([0, 3, 1]).with_flat());
    builder.push_face(Face::new([0, 2, 3]).with_flat());
    // right
    builder.push_face(Face::new([2, 7, 3]).with_flat());
    builder.push_face(Face::new([2, 6, 7]).with_flat());
    // back
    builder.push_face(Face::new([7, 6, 5]).with_flat());
    builder.push_face(Face::new([5, 6, 4]).with_flat());
    // left
    builder.push_face(Face::new([1, 5, 4]).with_flat());
    builder.push_face(Face::new([1, 4, 0]).with_flat());
    // top
    builder.push_face(Face::new([3, 5, 1]).with_flat());
    builder.push_face(Face::new([3, 7, 5]).with_flat());
    // bottom
    builder.push_face(Face::new([2, 0, 4]).with_flat());
    builder.push_face(Face::new([2, 4, 6]).with_flat());
    builder.build()
}

fn main() {
    //let mesh = build_cube();
    let mesh = build_solid();
    gltf::export("test.glb", &mesh).unwrap();
}
