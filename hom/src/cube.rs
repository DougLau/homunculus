use crate::mesh::{MeshBuilder, Tri};
use bevy::render::mesh::Mesh;
use glam::Vec3;

/// Build a cube mesh
pub fn build_cube() -> Mesh {
    let mut builder = MeshBuilder::new();
    let v0 = Vec3::new(-0.5, -0.5, 0.5); // 0 left bottom front
    let v1 = Vec3::new(-0.5, 0.5, 0.5); // 1 left top front
    let v2 = Vec3::new(0.5, -0.5, 0.5); // 2 right bottom front
    let v3 = Vec3::new(0.5, 0.5, 0.5); // 3 right top front
    let v4 = Vec3::new(-0.5, -0.5, -0.5); // 4 left bottom back
    let v5 = Vec3::new(-0.5, 0.5, -0.5); // 5 left top back
    let v6 = Vec3::new(0.5, -0.5, -0.5); // 6 right bottom back
    let v7 = Vec3::new(0.5, 0.5, -0.5); // 7 right top back

    // front
    builder.push_tri(Tri::new(v0, v3, v1));
    builder.push_tri(Tri::new(v0, v2, v3));
    // right
    builder.push_tri(Tri::new(v2, v7, v3));
    builder.push_tri(Tri::new(v2, v6, v7));
    // back
    builder.push_tri(Tri::new(v7, v6, v5));
    builder.push_tri(Tri::new(v5, v6, v4));
    // left
    builder.push_tri(Tri::new(v1, v5, v4));
    builder.push_tri(Tri::new(v1, v4, v0));
    // top
    builder.push_tri(Tri::new(v3, v5, v1));
    builder.push_tri(Tri::new(v3, v7, v5));
    // bottom
    builder.push_tri(Tri::new(v2, v0, v4));
    builder.push_tri(Tri::new(v2, v4, v6));

    builder.build()
}
