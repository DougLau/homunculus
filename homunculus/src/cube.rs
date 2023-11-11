// cube.rs      Cube module
//
// Copyright (c) 2022-2023  Douglas Lau
//
use crate::mesh::{Face, Mesh};
use glam::Vec3;

/// Build a cube mesh
pub fn build_cube() -> Mesh {
    let mut builder = Mesh::builder();
    builder.push_vtx(Vec3::new(-0.5, -0.5, 0.5)); // 0 left bottom front
    builder.push_vtx(Vec3::new(-0.5, 0.5, 0.5)); // 1 left top front
    builder.push_vtx(Vec3::new(0.5, -0.5, 0.5)); // 2 right bottom front
    builder.push_vtx(Vec3::new(0.5, 0.5, 0.5)); // 3 right top front
    builder.push_vtx(Vec3::new(-0.5, -0.5, -0.5)); // 4 left bottom back
    builder.push_vtx(Vec3::new(-0.5, 0.5, -0.5)); // 5 left top back
    builder.push_vtx(Vec3::new(0.5, -0.5, -0.5)); // 6 right bottom back
    builder.push_vtx(Vec3::new(0.5, 0.5, -0.5)); // 7 right top back

    // front
    builder.push_face(Face::new([0, 3, 1], 0.0));
    builder.push_face(Face::new([0, 2, 3], 0.0));
    // right
    builder.push_face(Face::new([2, 7, 3], 0.0));
    builder.push_face(Face::new([2, 6, 7], 0.0));
    // back
    builder.push_face(Face::new([7, 6, 5], 0.0));
    builder.push_face(Face::new([5, 6, 4], 0.0));
    // left
    builder.push_face(Face::new([1, 5, 4], 0.0));
    builder.push_face(Face::new([1, 4, 0], 0.0));
    // top
    builder.push_face(Face::new([3, 5, 1], 0.0));
    builder.push_face(Face::new([3, 7, 5], 0.0));
    // bottom
    builder.push_face(Face::new([2, 0, 4], 0.0));
    builder.push_face(Face::new([2, 4, 6], 0.0));
    builder.build()
}
