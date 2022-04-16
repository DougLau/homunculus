use crate::mesh::Vec3;
use gltf::binary::{Glb, Header};
use gltf_json::{
    accessor::{ComponentType, GenericComponentType, Type},
    buffer::{Target, View},
    mesh::{Mode, Primitive, Semantic},
    serialize,
    validation::Checked::Valid,
    {Accessor, Buffer, Index, Mesh, Node, Root, Scene, Value},
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::mem::size_of;

struct BufferBuilder {
    index: Index<Buffer>,
    bin: Vec<u8>,
}

fn align_to_four(n: usize) -> u32 {
    (n as u32 + 3) % 4
}

fn as_u8_slice<T: Sized>(p: &[T]) -> &[u8] {
    let (_head, body, _tail) = unsafe { p.align_to::<u8>() };
    body
}

impl BufferBuilder {
    fn new(index: u32) -> BufferBuilder {
        let index = Index::new(index);
        let bin = vec![];
        BufferBuilder { index, bin }
    }
    fn push_view<V>(&mut self, buf: &[V]) -> View {
        while self.bin.len() % 3 != 0 {
            self.bin.push(0);
        }
        let byte_offset = Some(self.bin.len().try_into().unwrap());
        let bytes = as_u8_slice(buf);
        let byte_length = bytes.len().try_into().unwrap();
        self.bin.extend_from_slice(bytes);
        View {
            buffer: self.index,
            byte_length,
            byte_offset,
            byte_stride: Some(size_of::<V>() as u32),
            target: Some(Valid(Target::ArrayBuffer)),
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        }
    }
    fn build(self) -> (Buffer, Vec<u8>) {
        let byte_length = self.bin.len().try_into().unwrap();
        (
            Buffer {
                byte_length,
                name: None,
                uri: None,
                extensions: Default::default(),
                extras: Default::default(),
            },
            self.bin,
        )
    }
}

pub fn export(filename: &str, positions: &[Vec3], normals: &[Vec3]) {
    assert_eq!(positions.len(), normals.len());
    let count = positions.len() as u32;
    let min = positions
        .iter()
        .map(|v| *v)
        .reduce(|min, v| v.min(min))
        .unwrap();
    let max = positions
        .iter()
        .map(|v| *v)
        .reduce(|max, v| v.max(max))
        .unwrap();
    let mut builder = BufferBuilder::new(0);
    let pos_view = builder.push_view(positions);
    let norm_view = builder.push_view(normals);
    let (buffer, bin) = builder.build();
    let buffer_len: u32 = bin.len().try_into().unwrap();
    let bin = Some(Cow::Owned(bin));
    let pos_accessor = Accessor {
        buffer_view: Some(Index::new(0)),
        byte_offset: 0,
        count,
        component_type: Valid(GenericComponentType(ComponentType::F32)),
        type_: Valid(Type::Vec3),
        min: Some(Value::from(&min.0[..])),
        max: Some(Value::from(&max.0[..])),
        name: None,
        normalized: false,
        sparse: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    let norm_accessor = Accessor {
        buffer_view: Some(Index::new(1)),
        byte_offset: 0,
        count,
        component_type: Valid(GenericComponentType(ComponentType::F32)),
        type_: Valid(Type::Vec3),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    let primitive = Primitive {
        attributes: {
            let mut map = HashMap::new();
            map.insert(Valid(Semantic::Positions), Index::new(0));
            map.insert(Valid(Semantic::Normals), Index::new(1));
            map
        },
        mode: Valid(Mode::Triangles),
        indices: None,
        material: None,
        targets: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    let mesh = Mesh {
        primitives: vec![primitive],
        name: None,
        weights: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    let node = Node {
        mesh: Some(Index::new(0)),
        camera: None,
        children: None,
        matrix: None,
        name: None,
        translation: None,
        rotation: None,
        scale: None,
        skin: None,
        weights: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    let root = Root {
        buffers: vec![buffer],
        buffer_views: vec![pos_view, norm_view],
        accessors: vec![pos_accessor, norm_accessor],
        meshes: vec![mesh],
        nodes: vec![node],
        scenes: vec![Scene {
            nodes: vec![Index::new(0)],
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        }],
        ..Default::default()
    };
    let root_json =
        serialize::to_string(&root).expect("JSON serialization error");
    let root_len = align_to_four(root_json.len());
    let glb = Glb {
        header: Header {
            magic: *b"glTF",
            version: 2,
            length: root_len + buffer_len,
        },
        json: Cow::Owned(root_json.into_bytes()),
        bin,
    };
    let writer = File::create(filename).expect("I/O error");
    glb.to_writer(writer).expect("glTF export error");
}
