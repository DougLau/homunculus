// cube.rs      Cube module
//
// Copyright (c) 2022-2023  Douglas Lau
//
use crate::mesh::{Face, Mesh, Smoothing};
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
    builder.push_face(Face::new([0, 3, 1], Smoothing::Flat));
    builder.push_face(Face::new([0, 2, 3], Smoothing::Flat));
    // right
    builder.push_face(Face::new([2, 7, 3], Smoothing::Flat));
    builder.push_face(Face::new([2, 6, 7], Smoothing::Flat));
    // back
    builder.push_face(Face::new([7, 6, 5], Smoothing::Flat));
    builder.push_face(Face::new([5, 6, 4], Smoothing::Flat));
    // left
    builder.push_face(Face::new([1, 5, 4], Smoothing::Flat));
    builder.push_face(Face::new([1, 4, 0], Smoothing::Flat));
    // top
    builder.push_face(Face::new([3, 5, 1], Smoothing::Flat));
    builder.push_face(Face::new([3, 7, 5], Smoothing::Flat));
    // bottom
    builder.push_face(Face::new([2, 0, 4], Smoothing::Flat));
    builder.push_face(Face::new([2, 4, 6], Smoothing::Flat));
    builder.build()
}
