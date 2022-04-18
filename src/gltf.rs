use crate::mesh::Mesh;
use serde_json::{json, Value};
use serde_repr::Serialize_repr;
use std::fs::File;
use std::io::Write;
use std::mem::size_of;

#[derive(Serialize_repr)]
#[repr(u32)]
enum ComponentType {
    U16 = 5123,
    F32 = 5126,
}

#[derive(Serialize_repr)]
#[repr(u32)]
enum Target {
    ArrayBuffer = 34962,
    ElementArrayBuffer = 34963,
}

struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
    meshes: Vec<Value>,
}

struct Glb {
    writer: File,
}

fn as_u8_slice<T: Sized>(p: &[T]) -> &[u8] {
    let (_head, body, _tail) = unsafe { p.align_to::<u8>() };
    body
}

impl Builder {
    fn new() -> Builder {
        let bin = vec![];
        let views = vec![];
        let accessors = vec![];
        let meshes = vec![];
        Builder {
            bin,
            views,
            accessors,
            meshes,
        }
    }
    fn add_mesh(&mut self, mesh: &Mesh) {
        let count = mesh.positions().len();
        let idx_view = self.views.len();
        self.accessors.push(json!({
            "bufferView": idx_view,
            "componentType": ComponentType::U16,
            "type": "SCALAR",
            "count": mesh.indices().len(),
        }));
        let v = self.push_view(mesh.indices(), Target::ElementArrayBuffer);
        self.views.push(v);

        let pos_view = self.views.len();
        self.accessors.push(json!({
            "bufferView": pos_view,
            "componentType": ComponentType::F32,
            "type": "VEC3",
            "count": count,
            "min": mesh.pos_min(),
            "max": mesh.pos_max(),
        }));
        let v = self.push_view(mesh.positions(), Target::ArrayBuffer);
        self.views.push(v);

        let norm_view = self.views.len();
        self.accessors.push(json!({
            "bufferView": norm_view,
            "componentType": ComponentType::F32,
            "type": "VEC3",
            "count": count,
        }));
        let v = self.push_view(mesh.normals(), Target::ArrayBuffer);
        self.views.push(v);

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
    fn push_view<V>(&mut self, buf: &[V], target: Target) -> Value {
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
            "target": target,
        })
    }
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
    fn bin(&self) -> &[u8] {
        &self.bin
    }
}

pub fn export(filename: &str, mesh: &Mesh) -> Result<(), std::io::Error> {
    let mut builder = Builder::new();
    builder.add_mesh(mesh);
    let bin = builder.bin();
    let mut root_json = builder.json().to_string();
    while root_json.len() % 4 != 0 {
        root_json.push(' ');
    }
    let mut glb = Glb::create(filename)?;
    glb.write_header((root_json.len() + bin.len()).try_into().unwrap())?;
    glb.write_json(&root_json)?;
    glb.write_bin(&bin)?;
    Ok(())
}

impl Glb {
    fn create(filename: &str) -> Result<Glb, std::io::Error> {
        let writer = File::create(filename)?;
        Ok(Glb { writer })
    }
    fn write_header(&mut self, len: u32) -> Result<(), std::io::Error> {
        self.writer.write(b"glTF")?;
        self.writer.write(&2u32.to_le_bytes())?;
        self.writer.write(&len.to_le_bytes())?;
        Ok(())
    }
    fn write_chunk(
        &mut self,
        ctype: &[u8],
        data: &[u8],
    ) -> Result<(), std::io::Error> {
        let len: u32 = data.len().try_into().unwrap();
        self.writer.write(&len.to_le_bytes())?;
        self.writer.write(ctype)?;
        self.writer.write(data)?;
        Ok(())
    }
    fn write_json(&mut self, json: &str) -> Result<(), std::io::Error> {
        self.write_chunk(b"JSON", json.as_bytes())
    }
    fn write_bin(&mut self, bin: &[u8]) -> Result<(), std::io::Error> {
        self.write_chunk(b"BIN\0", bin)
    }
}
