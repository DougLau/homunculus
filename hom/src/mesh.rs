use bevy::render::mesh::{Indices, Mesh};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
use glam::Vec3;

/// Triangle for mesh
#[derive(Clone, Copy, Debug)]
pub struct Tri {
    pos: [Vec3; 3],
    norm: [Vec3; 3],
}

/// Builder for bevy Mesh with TriangleList primitives
#[derive(Default)]
pub struct MeshBuilder {
    pos: Vec<[f32; 3]>,
    norm: Vec<[f32; 3]>,
    indices: Vec<u16>,
}

impl Tri {
    /// Create a new flat-shaded triangle
    pub fn new(p0: Vec3, p1: Vec3, p2: Vec3) -> Self {
        let pos = [p0, p1, p2];
        let norm = [(p0 - p1).cross(p0 - p2).normalize(); 3];
        Tri { pos, norm }
    }
}

impl MeshBuilder {
    /// Create a new mesh builder
    pub fn new() -> Self {
        MeshBuilder::default()
    }

    /// Push one vertex
    fn push_vtx(&mut self, pos: Vec3, norm: Vec3) {
        let idx = self.pos.len().try_into().unwrap();
        self.indices.push(idx);
        self.pos.push(*pos.as_ref());
        self.norm.push(*norm.as_ref());
    }

    /// Push one triangle face
    pub fn push_tri(&mut self, tri: Tri) {
        self.push_vtx(tri.pos[0], tri.norm[0]);
        self.push_vtx(tri.pos[1], tri.norm[1]);
        self.push_vtx(tri.pos[2], tri.norm[2]);
    }

    /// Build the mesh
    pub fn build(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.norm);
        mesh.insert_indices(Indices::U16(self.indices));
        mesh
    }
}
