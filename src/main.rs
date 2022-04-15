pub mod glb;
pub mod mesh;

fn main() {
    let vertices = vec![
        glb::VtxPosNorm {
            pos: [0.0, 0.5, 0.0],
            norm: [1.0, 0.0, 0.0],
        },
        glb::VtxPosNorm {
            pos: [-0.5, -0.5, 0.0],
            norm: [0.0, 1.0, 0.0],
        },
        glb::VtxPosNorm {
            pos: [0.5, -0.5, 0.0],
            norm: [0.0, 0.0, 1.0],
        },
    ];
    glb::export("test.glb", &vertices);
}
