pub mod mesh;
pub mod scene;

use mesh::{Face, MeshBuilder, Vec3};

fn main() {
    let mut builder = MeshBuilder::with_capacity(16);
    builder.push_vtx(Vec3([-0.5, -0.5, 0.5]));
    builder.push_vtx(Vec3([0.5, -0.5, 0.5]));
    builder.push_vtx(Vec3([0.5, 0.5, 0.5]));
    builder.push_vtx(Vec3([-0.5, 0.5, 0.5]));
    builder.push_face(Face::new([0, 1, 2]).with_flat());
    builder.push_face(Face::new([0, 2, 3]).with_flat());
    builder.push_vtx(Vec3([0.5, -0.5, -0.5]));
    builder.push_vtx(Vec3([0.5, 0.5, -0.5]));
    builder.push_face(Face::new([1, 4, 5]).with_flat());
    builder.push_face(Face::new([2, 1, 5]).with_flat());
    builder.push_vtx(Vec3([-0.5, 0.5, -0.5]));
    builder.push_face(Face::new([2, 5, 6]).with_flat());
    builder.push_face(Face::new([2, 6, 3]).with_flat());
    let mesh = builder.build();
    scene::export("test.glb", &mesh);
}
