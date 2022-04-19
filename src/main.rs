pub mod gltf;
pub mod mesh;

use mesh::{Face, Mesh, MeshBuilder, Vec3};
use std::collections::VecDeque;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    angle: f32,
    ring: usize,
    vertex: usize,
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

    fn add_ring(&mut self, ring: usize, count: usize) {
        let y = ring as f32;
        for i in 0..count {
            let angle = PI * 2.0 * i as f32 / count as f32;
            let vertex = self.builder.vertices();
            self.points.push(Point {
                angle,
                ring,
                vertex,
            });
            let x = angle.sin();
            let z = angle.cos();
            self.builder.push_vtx(Vec3([x, y, z]));
        }
    }

    fn make_band(&mut self, ring0: usize, ring1: usize) {
        let mut ring = VecDeque::new();
        for point in &self.points {
            if point.ring == ring0 || point.ring == ring1 {
                ring.push_back(point);
            }
        }
        ring.make_contiguous()
            .sort_by(|a, b| a.angle.partial_cmp(&b.angle).unwrap());
        let mut ipt = ring.pop_front().unwrap();
        let mut jpt = ring.pop_front().unwrap();
        assert!(ipt != jpt);
        if jpt.ring > ipt.ring {
            (ipt, jpt) = (jpt, ipt);
        }
        let (apt, bpt) = (ipt, jpt);
        while let Some(pt) = ring.pop_front() {
            self.builder.push_face(
                Face::new([ipt.vertex, jpt.vertex, pt.vertex]).with_flat(),
            );
            if pt.ring == ipt.ring {
                ipt = pt;
            } else {
                jpt = pt;
            }
        }
        // Connect with first vertices
        self.builder.push_face(
            Face::new([ipt.vertex, jpt.vertex, bpt.vertex]).with_flat(),
        );
        self.builder.push_face(
            Face::new([bpt.vertex, apt.vertex, ipt.vertex]).with_flat(),
        );
    }
}

fn build_solid() -> Mesh {
    let mut solid = SolidBuilder::new();
    solid.add_ring(0, 8);
    solid.add_ring(1, 6);
    solid.add_ring(2, 5);
    solid.add_ring(3, 4);
    solid.add_ring(4, 3);
    solid.make_band(0, 1);
    solid.make_band(1, 2);
    solid.make_band(2, 3);
    solid.make_band(3, 4);
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
