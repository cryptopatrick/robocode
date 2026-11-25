//! Round-trip tests: export recordings, read the `.glb` back with the `gltf` crate.

use cargo_robocode::glb::{export, replay::parse, space::Space};

const OPEN_ARENA: &str =
    include_str!("../../tests/fixtures/open_arena_0_5_0.json");
const GINZA: &str = include_str!("fixtures/ginzabattle02.json");

fn node<'a>(doc: &'a gltf::Document, name: &str) -> gltf::Node<'a> {
    doc.nodes()
        .find(|n| n.name() == Some(name))
        .unwrap_or_else(|| panic!("no node {name}"))
}

fn translations(
    doc: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    name: &str,
) -> Vec<[f32; 3]> {
    let anim = doc.animations().next().unwrap();
    let channel = anim
        .channels()
        .find(|c| {
            c.target().node().name() == Some(name)
                && c.target().property()
                    == gltf::animation::Property::Translation
        })
        .unwrap();
    match channel.reader(|b| Some(&buffers[b.index()])).read_outputs().unwrap()
    {
        gltf::animation::util::ReadOutputs::Translations(t) => t.collect(),
        _ => unreachable!(),
    }
}

fn close(a: [f32; 3], b: [f32; 3], eps: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < eps)
}

#[test]
fn exports_version_1_fixture() {
    let bytes = export(OPEN_ARENA).unwrap();
    let turns = parse(OPEN_ARENA).unwrap().turns.len();
    let g = gltf::Gltf::from_slice(&bytes).unwrap();
    assert_eq!(g.animations().count(), 1);
    let anim = g.animations().next().unwrap();
    assert_eq!(anim.name(), Some("battle"));
    let robot0 = node(&g, "robot0").index();
    let channel = anim
        .channels()
        .find(|c| {
            c.target().node().index() == robot0
                && c.target().property()
                    == gltf::animation::Property::Translation
        })
        .unwrap();
    let input = channel.sampler().input();
    assert_eq!(input.count(), turns);
    let max = input.max().unwrap();
    let want = (turns - 1) as f64 / 30.0;
    assert!((max[0].as_f64().unwrap() - want).abs() < 1e-5);
}

#[test]
fn positions_round_trip() {
    let bytes = export(OPEN_ARENA).unwrap();
    let r = parse(OPEN_ARENA).unwrap();
    let (doc, buffers, _) = gltf::import_slice(&bytes).unwrap();
    let t = translations(&doc, &buffers, "robot0");
    let f = r.turns[5].robots.iter().find(|f| f.id == 0).unwrap();
    assert!(close(t[5], Space::new(None).point(f.x, f.y, 0.0), 1e-4));
}

#[test]
fn materials_and_extras() {
    let bytes = export(OPEN_ARENA).unwrap();
    let g = gltf::Gltf::from_slice(&bytes).unwrap();
    let names: Vec<_> =
        g.materials().filter_map(|m| m.name().map(str::to_string)).collect();
    assert!(
        names.contains(&"robot0".to_string())
            && names.contains(&"robot1".to_string())
    );
    let scene = g.scenes().next().unwrap();
    let extras: serde_json::Value =
        serde_json::from_str(scene.extras().as_ref().unwrap().get()).unwrap();
    assert_eq!(
        extras["robots"],
        serde_json::json!(["SafeShooter", "Circler"])
    );
}

#[test]
fn golden_bytes() {
    let bytes = export(OPEN_ARENA).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/open_arena.glb");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &bytes).unwrap();
        return;
    }
    let golden = std::fs::read(&path)
        .expect("run with UPDATE_GOLDEN=1 to create the golden file");
    assert!(bytes == golden, "export differs from {}", path.display());
}

#[test]
fn ginza_is_deterministic() {
    let a = export(GINZA).unwrap();
    assert_eq!(a, export(GINZA).unwrap());
    gltf::Gltf::from_slice(&a).unwrap();
}
