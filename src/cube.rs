use crate::geom::Vec3;
use crate::mesh::{Face, Mesh, MeshBuilder};

/// Build a cube mesh
pub fn build_cube() -> Mesh {
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
