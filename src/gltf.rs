use crate::mesh;
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

struct BufferBuilder {
    index: u32,
    bin: Vec<u8>,
}

fn as_u8_slice<T: Sized>(p: &[T]) -> &[u8] {
    let (_head, body, _tail) = unsafe { p.align_to::<u8>() };
    body
}

impl BufferBuilder {
    fn new(index: u32) -> BufferBuilder {
        let bin = vec![];
        BufferBuilder { index, bin }
    }
    fn push_view<V>(&mut self, buf: &[V], target: Target) -> Value {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let byte_offset = self.bin.len();
        let bytes = as_u8_slice(buf);
        self.bin.extend_from_slice(bytes);
        json!({
            "buffer": self.index,
            "byteLength": bytes.len(),
            "byteOffset": byte_offset,
            "byteStride": size_of::<V>(),
            "target": target,
        })
    }
    fn build(self) -> Vec<u8> {
        self.bin
    }
}

pub fn export(filename: &str, mesh: &mesh::Mesh) -> Result<(), std::io::Error> {
    let count = mesh.positions().len();
    let min = mesh
        .positions()
        .iter()
        .map(|v| *v)
        .reduce(|min, v| v.min(min))
        .unwrap();
    let max = mesh
        .positions()
        .iter()
        .map(|v| *v)
        .reduce(|max, v| v.max(max))
        .unwrap();
    let mut builder = BufferBuilder::new(0);
    let idx_view =
        builder.push_view(mesh.indices(), Target::ElementArrayBuffer);
    let pos_view = builder.push_view(mesh.positions(), Target::ArrayBuffer);
    let norm_view = builder.push_view(mesh.normals(), Target::ArrayBuffer);
    let bin = builder.build();
    let buffer = json!({
        "byteLength": bin.len(),
    });
    let accessors = json!(
        [{
            "bufferView": 0,
            "byteOffset": 0,
            "componentType": ComponentType::U16,
            "count": mesh.indices().len(),
            "type": "SCALAR",
        },
        {
            "bufferView": 1,
            "byteOffset": 0,
            "componentType": ComponentType::F32,
            "count": count,
            "type": "VEC3",
            "min": min,
            "max": max,
        },
        {
            "bufferView": 2,
            "byteOffset": 0,
            "count": count,
            "componentType": ComponentType::F32,
            "type": "VEC3",
        }]
    );
    let meshes = json!(
        [{
            "primitives": [{
                "attributes": {
                    "POSITION": 1,
                    "NORMAL": 2,
                },
                "indices": 0,
            }],
        }]
    );
    let root = json!({
        "asset": {
            "version": "2.0"
        },
        "buffers": [buffer],
        "bufferViews": [idx_view, pos_view, norm_view],
        "accessors": accessors,
        "meshes": meshes,
        "nodes": [{
            "mesh": 0
        }],
        "scenes": [{
            "nodes": [0]
        }],
    });
    let mut root_json = root.to_string();
    while root_json.len() % 4 != 0 {
        root_json.push(' ');
    }
    let mut glb = Glb::create(filename)?;
    glb.write_header((root_json.len() + bin.len()).try_into().unwrap())?;
    glb.write_json(&root_json)?;
    glb.write_bin(&bin)?;
    Ok(())
}

struct Glb {
    writer: File,
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
