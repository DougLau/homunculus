pub mod mesh;
pub mod scene;

use mesh::Vec3;

fn main() {
    let positions = vec![
        Vec3([0.0, 0.5, 0.0]),
        Vec3([-0.5, -0.5, 0.0]),
        Vec3([0.5, -0.5, 0.0]),
    ];
    let normals = vec![
        Vec3([1.0, 0.0, 0.0]),
        Vec3([0.0, 1.0, 0.0]),
        Vec3([0.0, 0.0, 1.0]),
    ];
    scene::export("test.glb", &positions, &normals);
}
