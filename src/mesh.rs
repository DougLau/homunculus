use core::ops::{AddAssign, Mul, Sub};
use serde_derive::Serialize;

/// Vector of 3 components
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Vec3(pub [f32; 3]);

/// Vertex index
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vertex(pub u16);

impl From<usize> for Vertex {
    fn from(v: usize) -> Self {
        Self(v.try_into().expect("Too many vertices"))
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.0[0] += rhs.x();
        self.0[1] += rhs.y();
        self.0[2] += rhs.z();
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self([self.x() - rhs.x(), self.y() - rhs.y(), self.z() - rhs.z()])
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self([self.x() * rhs, self.y() * rhs, self.z() * rhs])
    }
}

impl Vec3 {
    pub const fn x(self) -> f32 {
        self.0[0]
    }
    pub const fn y(self) -> f32 {
        self.0[1]
    }
    pub const fn z(self) -> f32 {
        self.0[2]
    }
    pub fn min(self, rhs: Self) -> Self {
        let x = self.x().min(rhs.x());
        let y = self.y().min(rhs.y());
        let z = self.z().min(rhs.z());
        Vec3([x, y, z])
    }
    pub fn max(self, rhs: Self) -> Self {
        let x = self.x().max(rhs.x());
        let y = self.y().max(rhs.y());
        let z = self.z().max(rhs.z());
        Vec3([x, y, z])
    }
    fn magnitude(self) -> f32 {
        (self.x() * self.x() + self.y() * self.y() + self.z() * self.z()).sqrt()
    }
    #[must_use]
    fn cross(self, rhs: Vec3) -> Vec3 {
        let x = self.y() * rhs.z() - self.z() * rhs.y();
        let y = self.z() * rhs.x() - self.x() * rhs.z();
        let z = self.x() * rhs.y() - self.y() * rhs.x();
        Vec3([x, y, z])
    }
    fn dot(self, rhs: Vec3) -> f32 {
        self.x() * rhs.x() + self.y() * rhs.y() + self.z() * rhs.z()
    }
    #[must_use]
    fn normalize(self) -> Vec3 {
        let mag = self.magnitude();
        if mag != 0.0 {
            self * (1.0 / mag)
        } else {
            self
        }
    }
    fn angle(self, rhs: Vec3) -> f32 {
        let mag = self.magnitude() * rhs.magnitude();
        if mag != 0.0 { self.dot(rhs) / mag } else { 0.0 }
            .min(1.0)
            .max(-1.0)
            .acos()
    }
}

/// Face edge smoothng
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Sharp,
    Smooth,
}

/// Triangle face
///
/// Vertices and edges:
/// ```
/// v0______v2
///   \ e2 /
/// e0 \  / e1
///     \/
///     v1
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Face {
    /// Vertex positions
    vtx: [usize; 3],
    /// Edge smoothing
    edge: [Edge; 3],
}

/// Mesh builder
#[derive(Default)]
pub struct MeshBuilder {
    /// Vertex positions
    pos: Vec<Vec3>,
    /// Triangle faces
    faces: Vec<Face>,
}

/// 3D Mesh
pub struct Mesh {
    /// Vertex positions
    pos: Vec<Vec3>,
    /// Vertex normals
    norm: Vec<Vec3>,
    /// Vertex indices
    indices: Vec<Vertex>,
}

impl Face {
    /// Create a new face
    pub fn new(vtx: [usize; 3]) -> Self {
        let edge = [Edge::Smooth; 3];
        Self { vtx, edge }
    }

    /// Make shading flat (not smooth)
    pub fn with_flat(mut self) -> Self {
        self.edge = [Edge::Sharp; 3];
        self
    }

    /// Check if a vertex is next to a sharp edge
    fn is_sharp_vertex(&self, idx: usize) -> bool {
        (self.vtx[0] == idx
            && (self.edge[0] == Edge::Sharp || self.edge[2] == Edge::Sharp))
            || (self.vtx[1] == idx
                && (self.edge[1] == Edge::Sharp || self.edge[0] == Edge::Sharp))
            || (self.vtx[2] == idx
                && (self.edge[2] == Edge::Sharp || self.edge[1] == Edge::Sharp))
    }
}

impl MeshBuilder {
    /// Create a mesh builder with capacity for N faces
    pub fn with_capacity(n_faces: usize) -> Self {
        let pos = Vec::with_capacity(n_faces * 3);
        let faces = Vec::with_capacity(n_faces * 3);
        MeshBuilder { pos, faces }
    }

    /// Push a vertex position
    pub fn push_vtx(&mut self, pos: Vec3) -> usize {
        let idx = self.pos.len();
        self.pos.push(pos);
        idx
    }

    /// Push a face
    pub fn push_face(&mut self, face: Face) {
        let idx = self.pos.len();
        if face.vtx[0] >= idx || face.vtx[1] >= idx || face.vtx[2] >= idx {
            panic!("Invalid vertex");
        }
        self.faces.push(face);
    }

    /// Build the mesh
    pub fn build(self) -> Mesh {
        Mesh::new(self.split_edge_seams())
    }

    /// Split vertices at edge seams
    fn split_edge_seams(mut self) -> Self {
        let vertices = self.pos.len();
        for idx in 0..vertices {
            while self.vertex_needs_split(idx) {
                self.split_vertex(idx);
            }
        }
        self
    }

    /// Check if a vertex needs splitting
    fn vertex_needs_split(&self, idx: usize) -> bool {
        let mut found = false;
        for face in &self.faces {
            if face.is_sharp_vertex(idx) {
                if found {
                    return true;
                }
                found = true;
            }
        }
        false
    }

    /// Split one vertex
    fn split_vertex(&mut self, idx: usize) {
        let pos = self.pos[idx];
        let i = self.push_vtx(pos);
        for face in &mut self.faces {
            if face.is_sharp_vertex(idx) {
                if face.vtx[0] == idx {
                    face.vtx[0] = i;
                } else if face.vtx[1] == idx {
                    face.vtx[1] = i;
                } else if face.vtx[2] == idx {
                    face.vtx[2] = i;
                }
                break;
            }
        }
    }

    /// Calculate normals for all vertices
    fn build_normals(&self) -> Vec<Vec3> {
        let vertices = self.pos.len();
        let mut norm = vec![Vec3::default(); vertices];
        for face in &self.faces {
            let vtx = [face.vtx[0], face.vtx[1], face.vtx[2]];
            let pos = [self.pos[vtx[0]], self.pos[vtx[1]], self.pos[vtx[2]]];
            let trin = (pos[0] - pos[1]).cross(pos[0] - pos[2]).normalize();
            let a0 = (pos[1] - pos[0]).angle(pos[2] - pos[0]);
            norm[vtx[0]] += trin * a0;
            let a1 = (pos[2] - pos[1]).angle(pos[0] - pos[1]);
            norm[vtx[1]] += trin * a1;
            let a2 = (pos[0] - pos[2]).angle(pos[1] - pos[2]);
            norm[vtx[2]] += trin * a2;
        }
        norm.iter().map(|n| n.normalize()).collect()
    }

    /// Build `Vec` of indices for all faces
    fn build_indices(&self) -> Vec<Vertex> {
        let mut indices = Vec::with_capacity(self.faces.len() * 3);
        for face in &self.faces {
            indices.push(face.vtx[0].into());
            indices.push(face.vtx[1].into());
            indices.push(face.vtx[2].into());
        }
        indices
    }
}

impl Mesh {
    /// Create a new mesh
    fn new(builder: MeshBuilder) -> Self {
        let norm = builder.build_normals();
        let indices = builder.build_indices();
        let pos = builder.pos;
        Mesh { pos, norm, indices }
    }

    /// Get slice of all vertex positions
    pub fn positions(&self) -> &[Vec3] {
        &self.pos[..]
    }

    /// Get slice of all vertex normals
    pub fn normals(&self) -> &[Vec3] {
        &self.norm[..]
    }

    /// Get slice of vertex indices for all triangles
    pub fn indices(&self) -> &[Vertex] {
        &self.indices[..]
    }
}
