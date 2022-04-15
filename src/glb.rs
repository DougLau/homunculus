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
use std::fs::File;
use std::mem::size_of;

pub trait Vtx {
    fn pos(&self) -> [f32; 3];
    fn norm_offset() -> Option<u32>;
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct VtxPosNorm {
    pub pos: [f32; 3],
    pub norm: [f32; 3],
}

impl Vtx for VtxPosNorm {
    fn pos(&self) -> [f32; 3] {
        self.pos
    }
    fn norm_offset() -> Option<u32> {
        Some(3 * (size_of::<f32>() as u32))
    }
}

fn align_to_four(n: usize) -> u32 {
    (n as u32 + 3) % 4
}

fn as_u8_slice<T: Sized>(p: &[T]) -> &[u8] {
    let (_head, body, _tail) = unsafe { p.align_to::<u8>() };
    body
}

pub fn export<V: Vtx>(filename: &str, vertices: &[V]) {
    let min = vertices
        .iter()
        .map(|v| v.pos())
        .reduce(|min, v| [min[0].min(v[0]), min[1].min(v[1]), min[2].min(v[2])])
        .unwrap();
    let max = vertices
        .iter()
        .map(|v| v.pos())
        .reduce(|max, v| [max[0].max(v[0]), max[1].max(v[1]), max[2].max(v[2])])
        .unwrap();
    let count = vertices.len() as u32;
    let byte_length = count * size_of::<V>() as u32;
    let bin = Some(Cow::Borrowed(as_u8_slice(vertices)));
    let buffer = Buffer {
        byte_length,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: None,
    };
    let buffer_view = View {
        buffer: Index::new(0),
        byte_length,
        byte_offset: None,
        byte_stride: Some(size_of::<V>() as u32),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(Target::ArrayBuffer)),
    };
    let positions = Accessor {
        buffer_view: Some(Index::new(0)),
        byte_offset: 0,
        count,
        component_type: Valid(GenericComponentType(ComponentType::F32)),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(Type::Vec3),
        min: Some(Value::from(vec![min[0], min[1], min[2]])),
        max: Some(Value::from(vec![max[0], max[1], max[2]])),
        name: None,
        normalized: false,
        sparse: None,
    };
    let normals = Accessor {
        buffer_view: Some(Index::new(0)),
        byte_offset: V::norm_offset().unwrap(),
        count,
        component_type: Valid(GenericComponentType(ComponentType::F32)),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(Type::Vec3),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    };
    let primitive = Primitive {
        attributes: {
            let mut map = std::collections::HashMap::new();
            map.insert(Valid(Semantic::Positions), Index::new(0));
            map.insert(Valid(Semantic::Normals), Index::new(1));
            map
        },
        extensions: Default::default(),
        extras: Default::default(),
        indices: None,
        material: None,
        mode: Valid(Mode::Triangles),
        targets: None,
    };
    let mesh = Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        primitives: vec![primitive],
        weights: None,
    };
    let node = Node {
        camera: None,
        children: None,
        extensions: Default::default(),
        extras: Default::default(),
        matrix: None,
        mesh: Some(Index::new(0)),
        name: None,
        rotation: None,
        scale: None,
        translation: None,
        skin: None,
        weights: None,
    };
    let root = Root {
        accessors: vec![positions, normals],
        buffers: vec![buffer],
        buffer_views: vec![buffer_view],
        meshes: vec![mesh],
        nodes: vec![node],
        scenes: vec![Scene {
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            nodes: vec![Index::new(0)],
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
            length: root_len + byte_length,
        },
        json: Cow::Owned(root_json.into_bytes()),
        bin,
    };
    let writer = File::create(filename).expect("I/O error");
    glb.to_writer(writer).expect("glTF export error");
}
