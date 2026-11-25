//! The arena (ground and walls) and the map: buildings, roads and street furniture.

use serde_json::{Value, json};

use super::{MeshRef, Node, Scene, Space};
use crate::glb::geometry::{extrude, strip};
use crate::glb::replay::Replay;
use crate::glb::space::yaw_toward;

/// Building height in metres when the map gives none (28 px at 4 px/m, as in the 3D viewer).
const FALLBACK_BUILDING_M: f64 = 7.0;
const GROUND_THICK_M: f32 = 0.05;
const WALL_THICK_M: f32 = 1.0;
const WALL_HIGH_M: f32 = 2.0;
const ROAD_TOP_M: f64 = 0.03;
const CROSSING_TOP_M: f32 = 0.04;
const CROSSING_THICK_M: f32 = 0.01;

/// `(material name, sRGB hex)`; the material is created the first time something uses it.
type Colour = (&'static str, &'static str);

const GROUND: Colour = ("ground", "#0f1e2e");
const WALL: Colour = ("wall", "#e9eef2");
const BUILDING: Colour = ("building", "#4a5866");
const ROAD: Colour = ("road", "#1e2b3a");
const CROSSING: Colour = ("crossing", "#e9eef2");
const SIGNAL: Colour = ("signal", "#c9d6e2");
const ENTRANCE: Colour = ("entrance", "#3fb6c9");

fn boxed(
    scene: &mut Scene,
    name: String,
    colour: Colour,
    at: [f32; 3],
    size: [f32; 3],
) -> usize {
    let material = scene.material(colour.0, colour.1);
    let mut n = Node::new(name);
    n.mesh = Some((MeshRef::Box, material));
    n.translation = at;
    n.scale = size;
    scene.add(n, None)
}

pub(crate) fn add_arena_and_map(
    scene: &mut Scene,
    replay: &Replay,
    space: &Space,
) {
    let (w, h) =
        (space.len(replay.width) as f32, space.len(replay.height) as f32);
    boxed(
        scene,
        "ground".into(),
        GROUND,
        [w / 2.0, -GROUND_THICK_M / 2.0, h / 2.0],
        [w, GROUND_THICK_M, h],
    );
    let (t, y) = (WALL_THICK_M, WALL_HIGH_M / 2.0);
    for (name, at, size) in [
        ("north", [w / 2.0, y, -t / 2.0], [w + 2.0 * t, WALL_HIGH_M, t]),
        ("south", [w / 2.0, y, h + t / 2.0], [w + 2.0 * t, WALL_HIGH_M, t]),
        ("west", [-t / 2.0, y, h / 2.0], [t, WALL_HIGH_M, h]),
        ("east", [w + t / 2.0, y, h / 2.0], [t, WALL_HIGH_M, h]),
    ] {
        boxed(scene, format!("wall/{name}"), WALL, at, size);
    }

    let map = &replay.map;
    if map.shapes.is_empty() {
        for (i, b) in map.buildings.iter().enumerate() {
            let height =
                map.heights.get(i).copied().unwrap_or(FALLBACK_BUILDING_M);
            let at = space.point(
                b[0] + b[2] / 2.0,
                b[1] + b[3] / 2.0,
                height / 2.0,
            );
            let size = [
                space.len(b[2]) as f32,
                height as f32,
                space.len(b[3]) as f32,
            ];
            boxed(scene, format!("building{i}"), BUILDING, at, size);
        }
    } else {
        for (i, shape) in map.shapes.iter().enumerate() {
            let ring: Vec<[f64; 2]> = shape
                .ring
                .iter()
                .map(|p| [space.len(p[0]), space.len(p[1])])
                .collect();
            let mesh = extrude(&ring, shape.h);
            if mesh.indices.is_empty() {
                // Fewer than two distinct points: nothing to draw, and an empty mesh is invalid glTF.
                continue;
            }
            scene.meshes.push(mesh);
            let mut n = Node::new(format!("building{i}"));
            n.mesh = Some((
                MeshRef::Own(scene.meshes.len() - 1),
                scene.material(BUILDING.0, BUILDING.1),
            ));
            scene.add(n, None);
        }
    }
    for (i, line) in map.road_lines.iter().enumerate() {
        if let Some(mesh) = strip(
            &line.points,
            line.width,
            (replay.width, replay.height),
            space,
            ROAD_TOP_M,
        ) {
            scene.meshes.push(mesh);
            let mut n = Node::new(format!("road{i}"));
            n.mesh = Some((
                MeshRef::Own(scene.meshes.len() - 1),
                scene.material(ROAD.0, ROAD.1),
            ));
            scene.add(n, None);
        }
    }
    for (i, r) in map.roads.iter().enumerate() {
        let at = space.point(
            r[0] + r[2] / 2.0,
            r[1] + r[3] / 2.0,
            ROAD_TOP_M / 2.0,
        );
        let size = [
            space.len(r[2]) as f32,
            ROAD_TOP_M as f32,
            space.len(r[3]) as f32,
        ];
        boxed(
            scene,
            format!("road{}", map.road_lines.len() + i),
            ROAD,
            at,
            size,
        );
    }

    let inside = |x: f64, y: f64| {
        (0.0..=replay.width).contains(&x) && (0.0..=replay.height).contains(&y)
    };
    for (i, c) in
        map.crossings.iter().enumerate().filter(|(_, c)| inside(c.x, c.y))
    {
        let len = space.len(c.len) as f32;
        let at = space.point(
            c.x,
            c.y,
            f64::from(CROSSING_TOP_M - CROSSING_THICK_M / 2.0),
        );
        let n = boxed(
            scene,
            format!("crossing{i}"),
            CROSSING,
            at,
            [0.3 * len, CROSSING_THICK_M, len],
        );
        // `angle` is the road's direction in radians; the zebra spans the road.
        let a = c.angle;
        scene.nodes[n].rotation = yaw_toward(-a.sin(), a.cos());
    }
    for (i, s) in
        map.signals.iter().enumerate().filter(|(_, s)| inside(s.x, s.y))
    {
        boxed(
            scene,
            format!("signal{i}"),
            SIGNAL,
            space.point(s.x, s.y, 2.5),
            [0.3, 5.0, 0.3],
        );
    }
    for (i, e) in
        map.entrances.iter().enumerate().filter(|(_, e)| inside(e.x, e.y))
    {
        let n = boxed(
            scene,
            format!("entrance{i}"),
            ENTRANCE,
            space.point(e.x, e.y, 1.0),
            [3.0, 2.0, 3.0],
        );
        scene.nodes[n].extras = Some(json!({ "name": e.name }));
    }

    let extras =
        scene.extras.as_object_mut().expect("scene extras are an object");
    if let Some(name) = &map.name {
        extras.insert("map_name".into(), json!(name));
    }
    if let Some(attribution) = &map.attribution {
        extras.insert("attribution".into(), json!(attribution));
    }
    if !map.labels.is_empty() {
        let labels: Vec<Value> = map
            .labels
            .iter()
            .map(|l| json!({ "text": l.text, "position": space.point(l.x, l.y, l.h), "h": l.h }))
            .collect();
        extras.insert("labels".into(), Value::Array(labels));
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::{MeshRef, Scene, build};
    use crate::glb::replay::{Crossing, Map, RoadLine, Shape2d, Signal};
    use crate::glb::space::{Space, rotate};

    fn close(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
    }

    fn with_map(map: Map) -> Scene {
        let mut r = replay(vec![turn(vec![
            frame(0, 100.0, 100.0, 0.0),
            frame(1, 700.0, 500.0, 0.0),
        ])]);
        r.map = map;
        build(&r)
    }

    fn count(s: &Scene, prefix: &str) -> usize {
        s.nodes
            .iter()
            .filter(|n| {
                n.name
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.parse::<usize>().is_ok())
            })
            .count()
    }

    #[test]
    fn ground_and_walls() {
        let s = with_map(Map::default());
        let g = &s.nodes[find(&s, "ground")];
        assert!(matches!(g.mesh, Some((MeshRef::Box, _))));
        assert!(close(&g.scale, &[200.0, 0.05, 150.0]));
        assert!(close(&g.translation, &[100.0, -0.025, 75.0]));
        for w in ["north", "south", "east", "west"] {
            find(&s, &format!("wall/{w}"));
        }
        let north = &s.nodes[find(&s, "wall/north")];
        assert!(close(&north.translation, &[100.0, 1.0, -0.5]));
        let east = &s.nodes[find(&s, "wall/east")];
        assert!(close(&east.translation, &[200.5, 1.0, 75.0]));
        assert_eq!(count(&s, "building") + count(&s, "road"), 0);
    }

    #[test]
    fn rect_buildings_use_heights() {
        let s = with_map(Map {
            buildings: vec![[10.0, 20.0, 30.0, 40.0], [0.0, 0.0, 4.0, 4.0]],
            heights: vec![12.0],
            ..Map::default()
        });
        let b = &s.nodes[find(&s, "building0")];
        assert!(close(&b.scale, &[7.5, 12.0, 10.0]));
        assert!(close(
            &b.translation,
            &Space::new(None).point(25.0, 40.0, 6.0)
        ));
        assert_eq!(s.nodes[find(&s, "building1")].scale[1], 7.0);
    }

    #[test]
    fn shapes_replace_rects() {
        let s = with_map(Map {
            buildings: vec![[10.0, 20.0, 30.0, 40.0], [0.0, 0.0, 4.0, 4.0]],
            shapes: vec![Shape2d {
                ring: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
                h: 33.0,
                id: "x".into(),
            }],
            ..Map::default()
        });
        assert_eq!(count(&s, "building"), 1);
        let Some((MeshRef::Own(m), _)) = s.nodes[find(&s, "building0")].mesh
        else {
            panic!("building0 has no own mesh")
        };
        let mesh = &s.meshes[m];
        let max_y =
            mesh.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        let max_x =
            mesh.positions.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert_eq!((max_y, max_x), (33.0, 10.0));
    }

    #[test]
    fn road_outside_arena_dropped() {
        let line = |y: f64| RoadLine {
            class: "primary".into(),
            width: 10.0,
            points: vec![[0.0, y], [800.0, y]],
        };
        let s = with_map(Map {
            road_lines: vec![line(50.0), line(-50.0), line(300.0)],
            ..Map::default()
        });
        find(&s, "road0");
        find(&s, "road2");
        assert_eq!(count(&s, "road"), 2);
    }

    #[test]
    fn rect_roads_follow_road_lines() {
        let line = RoadLine {
            class: "primary".into(),
            width: 10.0,
            points: vec![[0.0, 50.0], [800.0, 50.0]],
        };
        let s = with_map(Map {
            road_lines: vec![line],
            roads: vec![[0.0, 0.0, 800.0, 20.0]],
            ..Map::default()
        });
        let r = &s.nodes[find(&s, "road1")];
        assert!(close(&r.scale, &[200.0, 0.03, 5.0]));
    }

    #[test]
    fn crossing_pose() {
        // `angle` is the road's direction in radians, like atan2(dy, dx); `len` is the
        // road's width, so the zebra's long axis (local −Z) runs across the road.
        let s = with_map(Map {
            crossings: vec![
                Crossing { x: 100.0, y: 100.0, angle: 0.0, len: 40.0 },
                Crossing {
                    x: 200.0,
                    y: 100.0,
                    angle: std::f64::consts::FRAC_PI_2,
                    len: 40.0,
                },
            ],
            ..Map::default()
        });
        let c = &s.nodes[find(&s, "crossing0")];
        assert!(close(
            &c.translation,
            &Space::new(None).point(100.0, 100.0, 0.035)
        ));
        assert!(close(&c.scale, &[3.0, 0.01, 10.0]));
        // A road running east: the zebra spans it north–south (glTF ±Z).
        let d = rotate(c.rotation, [0.0, 0.0, -1.0]);
        assert!(d[0].abs() < 1e-5 && (d[2].abs() - 1.0).abs() < 1e-5, "{d:?}");
        // A road running south: the zebra spans it east–west (glTF ±X).
        let c = &s.nodes[find(&s, "crossing1")];
        let d = rotate(c.rotation, [0.0, 0.0, -1.0]);
        assert!(d[2].abs() < 1e-5 && (d[0].abs() - 1.0).abs() < 1e-5, "{d:?}");
    }

    #[test]
    fn degenerate_shapes_dropped() {
        let shape =
            |ring: Vec<[f64; 2]>| Shape2d { ring, h: 10.0, id: "d".into() };
        let s = with_map(Map {
            shapes: vec![
                shape(vec![]),
                shape(vec![[5.0, 5.0]]),
                shape(vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0]]),
            ],
            ..Map::default()
        });
        assert_eq!(count(&s, "building"), 1);
        for m in &s.meshes {
            assert!(!m.indices.is_empty());
        }
    }

    #[test]
    fn point_items_outside_dropped() {
        let s = with_map(Map {
            signals: vec![
                Signal { x: -5.0, y: 10.0 },
                Signal { x: 5.0, y: 10.0 },
            ],
            ..Map::default()
        });
        assert_eq!(count(&s, "signal"), 1);
        let sig = &s.nodes[find(&s, "signal1")];
        assert!(close(
            &sig.translation,
            &Space::new(None).point(5.0, 10.0, 2.5)
        ));
    }

    #[test]
    fn ginza_map() {
        let r = crate::glb::replay::parse(include_str!(
            "../../../tests/fixtures/ginzabattle02.json"
        ))
        .unwrap();
        let s = build(&r);
        let counts: Vec<_> =
            ["building", "road", "crossing", "signal", "entrance"]
                .iter()
                .map(|p| count(&s, p))
                .collect();
        assert_eq!(counts, [77, 166, 34, 7, 16]);
        assert_eq!(
            s.extras["attribution"].as_str(),
            r.map.attribution.as_deref()
        );
        assert_eq!(s.extras["map_name"], "Ginza, Tokyo");
        assert_eq!(s.extras["labels"].as_array().unwrap().len(), 27);
        assert_eq!(s.extras["labels"][0]["text"], "Yayoi Building");
    }
}
