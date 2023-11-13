// mesh.rs      Mesh module
//
// Copyright (c) 2022=2023  Douglas Lau
//
use glam::Vec3;

/// Vertex index
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vertex(pub u16);

impl From<usize> for Vertex {
    fn from(v: usize) -> Self {
        Self(v.try_into().expect("Too many vertices"))
    }
}

/// Triangle face
///
/// ```text
/// v0______v2
///   \    /
///    \  /
///     \/
///     v1
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Face {
    /// Vertex positions
    vtx: [usize; 3],

    /// Surface number, for shading
    surface: u16,
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
    pub fn new(vtx: [usize; 3], surface: u16) -> Self {
        debug_assert_ne!(vtx[0], vtx[1]);
        debug_assert_ne!(vtx[1], vtx[2]);
        debug_assert_ne!(vtx[2], vtx[0]);
        Self { vtx, surface }
    }

    /// Get surface number for a vertex
    fn vertex_surface(&self, idx: usize) -> Option<u16> {
        self.vtx.contains(&idx).then(|| self.surface)
    }

    /// Split a vertex
    fn split_vertex(&mut self, idx: usize, i: usize) {
        if self.vtx[0] == idx {
            self.vtx[0] = i;
        } else if self.vtx[1] == idx {
            self.vtx[1] = i;
        } else if self.vtx[2] == idx {
            self.vtx[2] = i;
        }
    }
}

impl MeshBuilder {
    /// Create a mesh builder with capacity for N faces
    fn with_capacity(n_faces: usize) -> Self {
        let pos = Vec::with_capacity(n_faces * 3);
        let faces = Vec::with_capacity(n_faces * 3);
        MeshBuilder { pos, faces }
    }

    /// Get a vertex
    pub fn vertex(&self, idx: usize) -> Vec3 {
        self.pos[idx]
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
        Mesh::new(self.split_vertices())
    }

    /// Split all non-smooth vertices
    fn split_vertices(mut self) -> Self {
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
        let mut surface = None;
        for face in &self.faces {
            let surf = face.vertex_surface(idx);
            if let (Some(s), Some(sf)) = (surf, surface) {
                if s != sf {
                    return true;
                }
            }
            surface = surf;
        }
        false
    }

    /// Split one vertex
    fn split_vertex(&mut self, idx: usize) {
        let mut surfaces = Vec::with_capacity(8);
        for face in &self.faces {
            if let Some(surf) = face.vertex_surface(idx) {
                if surfaces.is_empty() {
                    surfaces.push((surf, idx));
                } else if !surfaces.iter().any(|(s, _i)| surf == *s) {
                    surfaces.push((surf, 0));
                }
            }
        }
        let pos = self.pos[idx];
        for i in 0..surfaces.len() {
            if surfaces[i].1 == 0 {
                surfaces[i].1 = self.push_vtx(pos);
            }
        }
        for face in &mut self.faces {
            if let Some(surf) = face.vertex_surface(idx) {
                if let Some(i) =
                    surfaces.iter().find_map(|(s, i)| (surf == *s).then(|| i))
                {
                    if *i != idx {
                        face.split_vertex(idx, *i);
                    }
                }
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
            let a0 = (pos[1] - pos[0]).angle_between(pos[2] - pos[0]);
            norm[vtx[0]] += trin * a0;
            let a1 = (pos[2] - pos[1]).angle_between(pos[0] - pos[1]);
            norm[vtx[1]] += trin * a1;
            let a2 = (pos[0] - pos[2]).angle_between(pos[1] - pos[2]);
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
    /// Create a new mesh builder
    pub fn builder() -> MeshBuilder {
        MeshBuilder::with_capacity(1024)
    }

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

    /// Get slice of vertex/normal indices for all triangles
    pub fn indices(&self) -> &[Vertex] {
        &self.indices[..]
    }

    /// Get minimum position
    pub fn pos_min(&self) -> Vec3 {
        self.positions()
            .iter()
            .copied()
            .reduce(|min, v| v.min(min))
            .unwrap()
    }

    /// Get maximum position
    pub fn pos_max(&self) -> Vec3 {
        self.positions()
            .iter()
            .copied()
            .reduce(|max, v| v.max(max))
            .unwrap()
    }
}
