pub mod cube;
pub mod gltf;
pub mod mesh;
pub mod solid;

fn main() {
    let cfg: solid::Config = muon_rs::from_str(SOLID).unwrap();
    let mesh = cfg.build();
    gltf::export("test.glb", &mesh).unwrap();
}

const SOLID: &str = &r#"
ring:
  radius: 1.2
  count: 6
ring:
  radius: 1.0
ring:
ring:
  count: 12
  radius: 0.75
ring:
  count: 3
  radius: 0.2
ring:
  count: 1
  radius: 0.1
"#;
