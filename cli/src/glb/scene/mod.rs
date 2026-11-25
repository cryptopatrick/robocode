//! The glTF scene in memory: nodes, meshes, materials and the `battle` animation clip.

mod map;
mod robots;

use serde_json::{Value, json};

use super::geometry::Mesh;
use super::replay::Replay;
use super::space::{DEFAULT_PX_PER_M, Space, srgb_hex};

/// Which geometry a node draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeshRef {
    /// The shared unit box.
    Box,
    /// The shared unit sphere.
    Sphere,
    /// An index into `Scene::meshes`.
    Own(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    /// `(geometry, material index)`.
    pub mesh: Option<(MeshRef, usize)>,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub children: Vec<usize>,
    pub extras: Option<Value>,
}

impl Node {
    /// A node with the identity transform and no mesh.
    pub fn new(name: impl Into<String>) -> Node {
        Node {
            name: name.into(),
            mesh: None,
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            children: Vec::new(),
            extras: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Path {
    Translation,
    Rotation,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interp {
    Linear,
    Step,
}

/// One animation channel: `values` holds 3 (translation, scale) or 4 (rotation) floats per key.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub node: usize,
    pub path: Path,
    pub interp: Interp,
    pub times: Vec<f32>,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    pub meshes: Vec<Mesh>,
    /// `(name, linear RGBA)`.
    pub materials: Vec<(String, [f32; 4])>,
    /// All tracks belong to the one clip, `battle`.
    pub tracks: Vec<Track>,
    pub extras: Value,
}

impl Scene {
    /// The index of the material `name`, created from an sRGB hex colour the first time.
    pub fn material(&mut self, name: &str, hex: &str) -> usize {
        if let Some(i) = self.materials.iter().position(|(n, _)| n == name) {
            return i;
        }
        self.materials.push((name.to_string(), srgb_hex(hex)));
        self.materials.len() - 1
    }

    /// Adds a node under `parent` (or as a root) and returns its index.
    pub fn add(&mut self, node: Node, parent: Option<usize>) -> usize {
        let i = self.nodes.len();
        self.nodes.push(node);
        match parent {
            Some(p) => self.nodes[p].children.push(i),
            None => self.roots.push(i),
        }
        i
    }
}

/// Builds the whole scene for a recording.
pub fn build(replay: &Replay) -> Scene {
    let space = Space::new(replay.map.px_per_m);
    let mut scene = Scene {
        extras: json!({
            "battlefield": { "width": replay.width, "height": replay.height },
            "bullets": replay.bullet_mode,
            "tanks": replay.tanks,
            "robots": replay.robots.iter().map(|r| r.name.clone()).collect::<Vec<_>>(),
            "winner": replay.winner,
            "px_per_m": replay.map.px_per_m.unwrap_or(DEFAULT_PX_PER_M),
        }),
        ..Scene::default()
    };
    map::add_arena_and_map(&mut scene, replay, &space);
    robots::add_robots(&mut scene, replay, &space);
    robots::add_bullets(&mut scene, replay, &space);
    scene
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::glb::replay::{Events, RobotFrame, RobotInfo, Turn};

    pub fn frame(id: usize, x: f64, y: f64, heading: f64) -> RobotFrame {
        RobotFrame {
            id,
            x,
            y,
            heading,
            gun_heading: heading,
            radar_heading: heading,
            alive: true,
            radius: 18.0,
        }
    }

    pub fn turn(robots: Vec<RobotFrame>) -> Turn {
        Turn { robots, bullets: Vec::new(), events: Events::default() }
    }

    /// An 800×600 replay with robots A (id 0) and B (id 1) and no map.
    pub fn replay(turns: Vec<Turn>) -> Replay {
        Replay {
            width: 800.0,
            height: 600.0,
            robots: vec![
                RobotInfo { id: 0, name: "A".into() },
                RobotInfo { id: 1, name: "B".into() },
            ],
            bullet_mode: "realistic".into(),
            tanks: None,
            winner: None,
            map: Default::default(),
            turns,
        }
    }

    pub fn find(scene: &Scene, name: &str) -> usize {
        scene
            .nodes
            .iter()
            .position(|n| n.name == name)
            .unwrap_or_else(|| panic!("no node {name}"))
    }

    pub fn track(scene: &Scene, node: usize, path: Path) -> &Track {
        scene
            .tracks
            .iter()
            .find(|t| t.node == node && t.path == path)
            .unwrap_or_else(|| panic!("no {path:?} track on node {node}"))
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    #[test]
    fn scene_extras() {
        let r = replay(vec![turn(vec![
            frame(0, 100.0, 100.0, 0.0),
            frame(1, 700.0, 500.0, 0.0),
        ])]);
        let s = build(&r);
        assert_eq!(
            s.extras,
            json!({
                "battlefield": { "width": 800.0, "height": 600.0 },
                "bullets": "realistic",
                "tanks": null,
                "robots": ["A", "B"],
                "winner": null,
                "px_per_m": 4.0,
            })
        );
        assert_eq!(
            s.nodes[find(&s, "robot0")].extras,
            Some(json!({"name": "A", "slot": 0}))
        );
    }

    #[test]
    fn every_material_is_used() {
        let r = replay(vec![turn(vec![
            frame(0, 100.0, 100.0, 0.0),
            frame(1, 700.0, 500.0, 0.0),
        ])]);
        let s = build(&r);
        for (i, (name, _)) in s.materials.iter().enumerate() {
            assert!(
                s.nodes.iter().any(|n| n.mesh.is_some_and(|m| m.1 == i)),
                "material {name} unused"
            );
        }
    }

    #[test]
    fn same_replay_same_scene() {
        let r = replay(vec![
            turn(vec![
                frame(0, 100.0, 100.0, 0.0),
                frame(1, 700.0, 500.0, 0.0),
            ]),
            turn(vec![
                frame(0, 101.0, 100.0, 5.0),
                frame(1, 700.0, 501.0, 350.0),
            ]),
        ]);
        assert_eq!(build(&r), build(&r));
    }
}
