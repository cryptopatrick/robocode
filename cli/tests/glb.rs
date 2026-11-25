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

#[test]
fn ginza_acceptance() {
    let bytes = export(GINZA).unwrap();
    let (doc, buffers, _) = gltf::import_slice(&bytes).unwrap();
    let sp = Space::new(Some(4.0));
    let extras = |n: gltf::Node| -> serde_json::Value {
        serde_json::from_str(n.extras().as_ref().unwrap().get()).unwrap()
    };
    // (a) robots and their names
    for (i, name) in
        ["Candidate", "Circler", "Tactician", "SittingDuck"].iter().enumerate()
    {
        assert_eq!(extras(node(&doc, &format!("robot{i}")))["name"], *name);
    }
    // (b) the dead hold their death positions
    let r3 = translations(&doc, &buffers, "robot3");
    assert!(close(r3[65], sp.point(290.0, 490.0, 0.0), 1.0), "{:?}", r3[65]);
    assert!(r3[65..].iter().all(|&p| p == r3[65]));
    let r0 = translations(&doc, &buffers, "robot0");
    assert!(
        close(r0[209], sp.point(1128.0, 499.0, 0.0), 1.0),
        "{:?}",
        r0[209]
    );
    assert!(r0[209..].iter().all(|&p| p == r0[209]));
    // (c) the survivors' last positions
    assert!(close(
        translations(&doc, &buffers, "robot1")[2000],
        sp.point(525.0, 447.0, 0.0),
        1.0
    ));
    assert!(close(
        translations(&doc, &buffers, "robot2")[2000],
        sp.point(715.0, 263.0, 0.0),
        1.0
    ));
    // (d) 40 bullets, each hidden after its last turn, nothing past the clip
    let end = 2000.0f32 / 30.0;
    let bullets: Vec<_> = doc
        .nodes()
        .filter(|n| n.name().is_some_and(|s| s.starts_with("bullet")))
        .collect();
    assert_eq!(bullets.len(), 40);
    let anim = doc.animations().next().unwrap();
    for b in &bullets {
        let ch = anim
            .channels()
            .find(|c| {
                c.target().node().index() == b.index()
                    && c.target().property()
                        == gltf::animation::Property::Scale
            })
            .unwrap();
        let reader = ch.reader(|x| Some(&buffers[x.index()]));
        let times: Vec<f32> = reader.read_inputs().unwrap().collect();
        let gltf::animation::util::ReadOutputs::Scales(scales) =
            reader.read_outputs().unwrap()
        else {
            unreachable!()
        };
        let last = scales.last().unwrap();
        assert!(
            last == [0.0; 3] || *times.last().unwrap() <= end,
            "{:?}",
            b.name()
        );
        assert!(times.iter().all(|&t| t <= end + 1e-4));
    }
    // (e) the map
    let count = |p: &str| {
        doc.nodes()
            .filter(|n| {
                n.name()
                    .and_then(|s| s.strip_prefix(p))
                    .is_some_and(|r| r.parse::<usize>().is_ok())
            })
            .count()
    };
    let counts: Vec<_> =
        ["building", "road", "crossing", "signal", "entrance"]
            .iter()
            .map(|p| count(p))
            .collect();
    assert_eq!(counts, [77, 166, 34, 7, 16]);
    // (f) the clip ends at turn 2000
    let max = anim
        .samplers()
        .map(|s| s.input().max().unwrap()[0].as_f64().unwrap())
        .fold(0.0, f64::max);
    assert!((max - f64::from(end)).abs() < 1e-4, "{max}");
    // (g) a tank is 9 m long
    assert_eq!(node(&doc, "robot0/body").transform().decomposed().2[2], 9.0);
    // (h) no winner, and the map's credit travels with the file
    let scene: serde_json::Value = serde_json::from_str(
        doc.scenes().next().unwrap().extras().as_ref().unwrap().get(),
    )
    .unwrap();
    assert!(scene["winner"].is_null());
    assert!(scene["attribution"].as_str().unwrap().contains("PLATEAU"));
}
