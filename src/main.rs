pub mod cube;
pub mod gltf;
pub mod mesh;
pub mod solid;

fn main() {
    let mesh = solid::build_solid();
    gltf::export("test.glb", &mesh).unwrap();
}
