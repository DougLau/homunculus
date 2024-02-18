// gltf.rs      glTF module
//
// Copyright (c) 2022-2024  Douglas Lau
//
use crate::mesh::Mesh;
use serde_json::{json, Value};
use serde_repr::Serialize_repr;
use std::io::{Result, Write};
use std::mem::size_of;

/// Component types for glTF accessor
#[derive(Serialize_repr)]
#[repr(u32)]
#[allow(unused)]
enum ComponentType {
    I8 = 5120,
    U8 = 5121,
    I16 = 5122,
    U16 = 5123,
    U32 = 5125,
    F32 = 5126,
}

/// Target for glTF buffer view
#[derive(Serialize_repr)]
#[repr(u32)]
enum Target {
    ArrayBuffer = 34962,
    ElementArrayBuffer = 34963,
}

/// Builder for glTF
#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
    meshes: Vec<Value>,
}

/// GLB writer
struct Glb<W: Write> {
    writer: W,
}

/// Transmute a slice of `T` to a slice of `u8`
fn as_u8_slice<T: Sized>(p: &[T]) -> &[u8] {
    let (_head, body, _tail) = unsafe { p.align_to::<u8>() };
    body
}

impl Builder {
    /// Add a mesh
    fn add_mesh(&mut self, mesh: &Mesh) {
        let count = mesh.positions().len();
        // indices
        let idx_view = self.views.len();
        self.accessors.push(json!({
            "bufferView": idx_view,
            "componentType": ComponentType::U16,
            "type": "SCALAR",
            "count": mesh.indices().len(),
        }));
        let v = self.push_index_view(mesh.indices());
        self.views.push(v);
        // positions
        let pos_view = self.views.len();
        self.accessors.push(json!({
            "bufferView": pos_view,
            "componentType": ComponentType::F32,
            "type": "VEC3",
            "count": count,
            "min": mesh.pos_min(),
            "max": mesh.pos_max(),
        }));
        let v = self.push_array_view(mesh.positions());
        self.views.push(v);
        // normals
        let norm_view = self.views.len();
        self.accessors.push(json!({
            "bufferView": norm_view,
            "componentType": ComponentType::F32,
            "type": "VEC3",
            "count": count,
        }));
        let v = self.push_array_view(mesh.normals());
        self.views.push(v);
        // mesh
        self.meshes.push(json!({
            "primitives": [{
                "attributes": {
                    "POSITION": pos_view,
                    "NORMAL": norm_view,
                },
                "indices": idx_view,
            }],
        }));
    }

    /// Push an index view
    fn push_index_view<V>(&mut self, buf: &[V]) -> Value {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let byte_offset = self.bin.len();
        let bytes = as_u8_slice(buf);
        self.bin.extend_from_slice(bytes);
        // no byteStride for index view
        json!({
            "buffer": 0,
            "byteLength": bytes.len(),
            "byteOffset": byte_offset,
            "target": Target::ElementArrayBuffer,
        })
    }

    /// Push an array view
    fn push_array_view<V>(&mut self, buf: &[V]) -> Value {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let byte_offset = self.bin.len();
        let bytes = as_u8_slice(buf);
        self.bin.extend_from_slice(bytes);
        json!({
            "buffer": 0,
            "byteLength": bytes.len(),
            "byteOffset": byte_offset,
            "byteStride": size_of::<V>(),
            "target": Target::ArrayBuffer,
        })
    }

    /// Get root JSON of glTF
    fn json(&self) -> Value {
        json!({
            "asset": {
                "version": "2.0"
            },
            "buffers": [{
                "byteLength": self.bin.len(),
            }],
            "bufferViews": self.views,
            "accessors": self.accessors,
            "meshes": self.meshes,
            "nodes": [{
                "mesh": 0
            }],
            "scenes": [{
                "nodes": [0]
            }],
        })
    }

    /// Get binary buffer
    fn bin(&self) -> &[u8] {
        &self.bin
    }
}

/// Export a mesh to a writer as a GLB
pub fn export<W: Write>(writer: W, mesh: &Mesh) -> Result<()> {
    let mut builder = Builder::default();
    builder.add_mesh(mesh);
    let bin = builder.bin();
    let mut root_json = builder.json().to_string();
    while root_json.len() % 4 != 0 {
        root_json.push(' ');
    }
    let mut glb = Glb::new(writer);
    glb.write_header(2, (root_json.len() + bin.len()).try_into().unwrap())?;
    glb.write_json(&root_json)?;
    glb.write_bin(bin)?;
    Ok(())
}

impl<W: Write> Glb<W> {
    /// Create new GLB writer
    fn new(writer: W) -> Self {
        Glb { writer }
    }

    /// Write GLB header
    fn write_header(&mut self, chunks: u32, len: u32) -> Result<()> {
        let total_len = 12 + chunks * 8 + len;
        self.writer.write_all(b"glTF")?;
        self.writer.write_all(&2u32.to_le_bytes())?;
        self.writer.write_all(&total_len.to_le_bytes())?;
        Ok(())
    }

    /// Write one chunk
    fn write_chunk(&mut self, ctype: &[u8], data: &[u8]) -> Result<()> {
        let len: u32 = data.len().try_into().unwrap();
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(ctype)?;
        self.writer.write_all(data)?;
        Ok(())
    }

    /// Write a JSON chunk
    fn write_json(&mut self, json: &str) -> Result<()> {
        self.write_chunk(b"JSON", json.as_bytes())
    }

    /// Write a BIN chunk
    fn write_bin(&mut self, bin: &[u8]) -> Result<()> {
        self.write_chunk(b"BIN\0", bin)
    }
}
