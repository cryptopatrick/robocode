//! Writes a `Scene` as a glTF 2.0 binary (`.glb`): a JSON chunk and one binary buffer.
//!
//! The output depends only on the scene, so the same recording always gives the same bytes.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::geometry::{Mesh, unit_box, unit_sphere};
use super::scene::{Interp, MeshRef, Path, Scene};

const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;

/// The growing binary buffer with its buffer views and accessors.
#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Builder {
    fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while self.bin.len() & 3 != 0 {
            self.bin.push(0);
        }
        let mut v = json!({ "buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len() });
        if let Some(t) = target {
            v["target"] = json!(t);
        }
        self.bin.extend_from_slice(bytes);
        self.views.push(v);
        self.views.len() - 1
    }

    /// A float accessor of `kind` (`SCALAR`, `VEC3`, `VEC4`) with `width` floats per element.
    fn floats(
        &mut self,
        values: &[f32],
        width: usize,
        kind: &str,
        target: Option<u32>,
        bounds: bool,
    ) -> usize {
        let bytes: Vec<u8> =
            values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let view = self.view(&bytes, target);
        let mut a = json!({
            "bufferView": view,
            "componentType": FLOAT,
            "count": values.len() / width,
            "type": kind,
        });
        if bounds {
            let mut min = vec![f32::INFINITY; width];
            let mut max = vec![f32::NEG_INFINITY; width];
            for chunk in values.chunks(width) {
                for (i, &v) in chunk.iter().enumerate() {
                    min[i] = min[i].min(v);
                    max[i] = max[i].max(v);
                }
            }
            a["min"] = json!(min);
            a["max"] = json!(max);
        }
        self.accessors.push(a);
        self.accessors.len() - 1
    }

    fn indices(&mut self, indices: &[u32]) -> usize {
        let bytes: Vec<u8> =
            indices.iter().flat_map(|v| v.to_le_bytes()).collect();
        let view = self.view(&bytes, Some(ELEMENT_ARRAY_BUFFER));
        self.accessors.push(json!({
            "bufferView": view,
            "componentType": UNSIGNED_INT,
            "count": indices.len(),
            "type": "SCALAR",
        }));
        self.accessors.len() - 1
    }

    /// `(POSITION, NORMAL, TEXCOORD_0, indices)` accessors for a mesh.
    fn geometry(&mut self, mesh: &Mesh) -> (usize, usize, usize, usize) {
        let positions: Vec<f32> =
            mesh.positions.iter().flatten().copied().collect();
        let normals: Vec<f32> =
            mesh.normals.iter().flatten().copied().collect();
        let p = self.floats(&positions, 3, "VEC3", Some(ARRAY_BUFFER), true);
        let n = self.floats(&normals, 3, "VEC3", Some(ARRAY_BUFFER), false);
        let uvs: Vec<f32> = mesh.uvs.iter().flatten().copied().collect();
        let uv = self.floats(&uvs, 2, "VEC2", Some(ARRAY_BUFFER), false);
        let i = self.indices(&mesh.indices);
        (p, n, uv, i)
    }
}

fn is_identity(v: &[f32], identity: &[f32]) -> bool {
    v == identity
}

pub fn write_glb(scene: &Scene) -> Vec<u8> {
    let mut b = Builder::default();

    // Geometry: the shared shapes once each, then the scene's own meshes, in order of first use.
    let mut geometry: BTreeMap<GeomKey, (usize, usize, usize, usize)> =
        BTreeMap::new();
    let mut meshes: Vec<Value> = Vec::new();
    let mut mesh_of: BTreeMap<Vec<(GeomKey, usize)>, usize> = BTreeMap::new();
    let mut nodes: Vec<Value> = Vec::with_capacity(scene.nodes.len());
    for node in &scene.nodes {
        let mut n = json!({ "name": node.name });
        if !node.mesh.is_empty() {
            let mut prims = Vec::with_capacity(node.mesh.len());
            let mut key = Vec::with_capacity(node.mesh.len());
            for &(shape, material) in &node.mesh {
                let g = GeomKey::from(shape);
                let (p, nrm, uv, i) =
                    *geometry.entry(g).or_insert_with(|| {
                        let mesh = match shape {
                            MeshRef::Box => unit_box(),
                            MeshRef::Sphere => unit_sphere(),
                            MeshRef::Own(i) => scene.meshes[i].clone(),
                        };
                        b.geometry(&mesh)
                    });
                key.push((g, material));
                prims.push(json!({
                    "attributes": { "POSITION": p, "NORMAL": nrm, "TEXCOORD_0": uv },
                    "indices": i,
                    "material": material,
                }));
            }
            let index = *mesh_of.entry(key).or_insert_with(|| {
                meshes.push(json!({ "primitives": prims }));
                meshes.len() - 1
            });
            n["mesh"] = json!(index);
        }
        if !is_identity(&node.translation, &[0.0; 3]) {
            n["translation"] = json!(node.translation);
        }
        if !is_identity(&node.rotation, &[0.0, 0.0, 0.0, 1.0]) {
            n["rotation"] = json!(node.rotation);
        }
        if !is_identity(&node.scale, &[1.0; 3]) {
            n["scale"] = json!(node.scale);
        }
        if !node.children.is_empty() {
            n["children"] = json!(node.children);
        }
        if let Some(extras) = &node.extras {
            n["extras"] = extras.clone();
        }
        nodes.push(n);
    }

    // Animation: tracks with identical key times share one input accessor.
    let mut inputs: BTreeMap<Vec<u32>, usize> = BTreeMap::new();
    let mut samplers = Vec::with_capacity(scene.tracks.len());
    let mut channels = Vec::with_capacity(scene.tracks.len());
    for track in &scene.tracks {
        let bits: Vec<u32> = track.times.iter().map(|t| t.to_bits()).collect();
        let input = match inputs.get(&bits) {
            Some(&i) => i,
            None => {
                let i = b.floats(&track.times, 1, "SCALAR", None, true);
                inputs.insert(bits, i);
                i
            }
        };
        let (width, kind, path) = match track.path {
            Path::Translation => (3, "VEC3", "translation"),
            Path::Rotation => (4, "VEC4", "rotation"),
            Path::Scale => (3, "VEC3", "scale"),
        };
        let output = b.floats(&track.values, width, kind, None, false);
        let interpolation = match track.interp {
            Interp::Linear => "LINEAR",
            Interp::Step => "STEP",
        };
        samplers.push(json!({ "input": input, "output": output, "interpolation": interpolation }));
        channels.push(json!({
            "sampler": samplers.len() - 1,
            "target": { "node": track.node, "path": path },
        }));
    }

    let materials: Vec<Value> = scene
        .materials
        .iter()
        .map(|(name, colour)| {
            json!({
                "name": name,
                "pbrMetallicRoughness": {
                    "baseColorFactor": colour,
                    "metallicFactor": 0.0,
                    "roughnessFactor": 0.9,
                },
            })
        })
        .collect();

    while b.bin.len() & 3 != 0 {
        b.bin.push(0);
    }
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "cargo-robocode" },
        "scene": 0,
        "scenes": [{ "nodes": scene.roots, "extras": scene.extras }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "accessors": b.accessors,
        "bufferViews": b.views,
        "buffers": [{ "byteLength": b.bin.len() }],
    });
    if !channels.is_empty() {
        doc["animations"] = json!([{ "name": "battle", "samplers": samplers, "channels": channels }]);
    }

    let mut json_chunk =
        serde_json::to_vec(&doc).expect("a JSON value always serializes");
    while json_chunk.len() & 3 != 0 {
        json_chunk.push(b' ');
    }
    let total = 12 + 8 + json_chunk.len() + 8 + b.bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&0x4654_6C67u32.to_le_bytes()); // "glTF"
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F_534Au32.to_le_bytes()); // "JSON"
    out.extend_from_slice(&json_chunk);
    out.extend_from_slice(&(b.bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E_4942u32.to_le_bytes()); // "BIN\0"
    out.extend_from_slice(&b.bin);
    out
}

/// `MeshRef` as an ordered key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum GeomKey {
    Box,
    Sphere,
    Own(usize),
}

impl From<MeshRef> for GeomKey {
    fn from(m: MeshRef) -> GeomKey {
        match m {
            MeshRef::Box => GeomKey::Box,
            MeshRef::Sphere => GeomKey::Sphere,
            MeshRef::Own(i) => GeomKey::Own(i),
        }
    }
}
