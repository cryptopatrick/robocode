//! Robots (body, gun, radar), tracked bullets and one-turn tracers.

use std::collections::BTreeMap;

use serde_json::json;

use super::{Interp, MeshRef, Node, Path, Scene, Space, Track};
use crate::glb::geometry::{self, Mesh};
use crate::glb::replay::{Replay, RobotFrame};
use crate::glb::space::{continuous, turn_time, yaw, yaw_toward};
use crate::glb::tanks::{self, Paint, Tank, TankModel, tank_for};

/// Slot colours, as in the viewer's `ROBOT_COLORS`.
const ROBOT_COLOURS: [&str; 4] = ["#e0a526", "#3fb6c9", "#ef5d79", "#9c7cf4"];
const BULLET_COLOUR: &str = "#fff6c8";
const DEFAULT_RADIUS_PX: f64 = 18.0;
/// A dead robot's height, as a share of its own (the viewer's `setDead`).
const WRECK_HEIGHT: f32 = 0.4;

/// A robot's diameter in metres, from its first recorded frame.
fn diameter(replay: &Replay, id: usize, space: &Space) -> f32 {
    let radius = replay
        .turns
        .iter()
        .flat_map(|t| &t.robots)
        .find(|f| f.id == id)
        .map_or(DEFAULT_RADIUS_PX, |f| f.radius);
    space.len(2.0 * radius) as f32
}

/// The diameter used for bullets: the first robot's.
fn bullet_diameter(replay: &Replay, space: &Space) -> f32 {
    replay
        .robots
        .first()
        .map_or(space.len(2.0 * DEFAULT_RADIUS_PX) as f32, |r| {
            diameter(replay, r.id, space)
        })
}

/// Each turn's pose for robot `id`. A missing turn repeats the previous pose, and the pose
/// is frozen from the first frame with `alive: false`.
fn poses(replay: &Replay, id: usize) -> Vec<RobotFrame> {
    let first = replay
        .turns
        .iter()
        .flat_map(|t| &t.robots)
        .find(|f| f.id == id)
        .cloned();
    let Some(mut current) = first else {
        return Vec::new();
    };
    let mut dead = false;
    replay
        .turns
        .iter()
        .map(|t| {
            if !dead && let Some(f) = t.robots.iter().find(|f| f.id == id) {
                current = f.clone();
                dead = !f.alive;
            }
            current.clone()
        })
        .collect()
}

fn times(n: usize) -> Vec<f32> {
    (0..n).map(turn_time).collect()
}

fn rotation_track(node: usize, headings: impl Iterator<Item = f64>) -> Track {
    let mut values = Vec::new();
    let mut prev: Option<[f32; 4]> = None;
    for h in headings {
        let q = prev.map_or(yaw(h), |p| continuous(p, yaw(h)));
        values.extend(q);
        prev = Some(q);
    }
    Track {
        node,
        path: Path::Rotation,
        interp: Interp::Linear,
        times: times(values.len() / 4),
        values,
    }
}

fn first_key<const N: usize>(t: &Track) -> [f32; N] {
    t.values[..N].try_into().expect("a track has at least one key")
}

/// The tank model each robot drives, by robot id.
fn models(
    replay: &Replay,
    space: &Space,
) -> BTreeMap<usize, (&'static Tank, TankModel)> {
    let header = replay.tanks.as_deref().unwrap_or(&[]);
    replay
        .robots
        .iter()
        .enumerate()
        .map(|(slot, r)| {
            let tank = tank_for(header, slot);
            (r.id, (tank, tanks::build(tank, space.len(1.0))))
        })
        .collect()
}

/// Geometry already in the scene for a tank's part list, keyed by tank id and part group.
type Shared = BTreeMap<(&'static str, &'static str), Vec<MeshRef>>;

/// A node drawing `parts`, painted with the tank's materials and the robot's `team` material.
/// Robots driving the same tank share its geometry; only the team material differs.
fn part_node(
    scene: &mut Scene,
    shared: &mut Shared,
    name: String,
    group: &'static str,
    parts: &[(Mesh, Paint)],
    tank: &'static Tank,
    team: usize,
) -> Node {
    let geometry = shared
        .entry((tank.id, group))
        .or_insert_with(|| {
            parts.iter().map(|(m, _)| scene.own(m.clone())).collect()
        })
        .clone();
    let mut n = Node::new(name);
    for ((_, paint), g) in parts.iter().zip(geometry) {
        let material = match paint {
            Paint::Team => team,
            p => {
                let (name, hex) = p.material(tank);
                scene.material(&name, &hex)
            }
        };
        n.mesh.push((g, material));
    }
    n
}

pub(crate) fn add_robots(scene: &mut Scene, replay: &Replay, space: &Space) {
    let models = models(replay, space);
    let mut shared = Shared::new();
    for (slot, info) in replay.robots.iter().enumerate() {
        let poses = poses(replay, info.id);
        if poses.is_empty() {
            continue;
        }
        let (tank, model) = &models[&info.id];
        let team = scene.material(
            &format!("robot{slot}"),
            ROBOT_COLOURS[slot % ROBOT_COLOURS.len()],
        );
        let name = format!("robot{}", info.id);

        let mut root = Node::new(&name);
        root.extras =
            Some(json!({ "name": info.name, "slot": slot, "tank": tank.id }));
        let root = scene.add(root, None);

        let hull = part_node(
            scene,
            &mut shared,
            format!("{name}/hull"),
            "hull",
            &model.hull,
            tank,
            team,
        );
        scene.add(hull, Some(root));
        let mut gun = part_node(
            scene,
            &mut shared,
            format!("{name}/gun"),
            "gun",
            &model.turret,
            tank,
            team,
        );
        gun.translation = model.gun_at;
        let gun = scene.add(gun, Some(root));
        let barrel = part_node(
            scene,
            &mut shared,
            format!("{name}/gun/barrel"),
            "barrel",
            &model.barrel,
            tank,
            team,
        );
        scene.add(barrel, Some(gun));
        let mut radar = part_node(
            scene,
            &mut shared,
            format!("{name}/radar"),
            "radar",
            &model.radar,
            tank,
            team,
        );
        radar.translation = model.radar_at;
        let radar = scene.add(radar, Some(root));

        let translation = Track {
            node: root,
            path: Path::Translation,
            interp: Interp::Linear,
            times: times(poses.len()),
            values: poses
                .iter()
                .flat_map(|p| space.point(p.x, p.y, 0.0))
                .collect(),
        };
        let body_rot = rotation_track(root, poses.iter().map(|p| p.heading));
        let gun_rot = rotation_track(
            gun,
            poses.iter().map(|p| p.gun_heading - p.heading),
        );
        let radar_rot = rotation_track(
            radar,
            poses.iter().map(|p| p.radar_heading - p.heading),
        );

        scene.nodes[root].translation = first_key(&translation);
        scene.nodes[root].rotation = first_key(&body_rot);
        scene.nodes[gun].rotation = first_key(&gun_rot);
        scene.nodes[radar].rotation = first_key(&radar_rot);
        scene.tracks.extend([translation, body_rot, gun_rot, radar_rot]);

        // A wreck is squashed to 40% of its height from the turn it dies.
        if let Some(death) = poses.iter().position(|p| !p.alive) {
            let squashed = [1.0, WRECK_HEIGHT, 1.0];
            let (times, values) = if death == 0 {
                (vec![0.0], squashed.to_vec())
            } else {
                (vec![0.0, turn_time(death)], [[1.0; 3], squashed].concat())
            };
            scene.nodes[root].scale = first_key(&Track {
                node: root,
                path: Path::Scale,
                interp: Interp::Step,
                times: times.clone(),
                values: values.clone(),
            });
            scene.tracks.push(Track {
                node: root,
                path: Path::Scale,
                interp: Interp::Step,
                times,
                values,
            });
        }
    }
}

/// A STEP scale track: full size (1) on turns `first..=last` and 0 otherwise.
fn visibility(node: usize, first: usize, last: usize, turns: usize) -> Track {
    let mut times = Vec::new();
    let mut values = Vec::new();
    if first > 0 {
        times.push(0.0);
        values.extend([0.0; 3]);
    }
    times.push(turn_time(first));
    values.extend([1.0; 3]);
    if last + 1 < turns {
        times.push(turn_time(last + 1));
        values.extend([0.0; 3]);
    }
    Track { node, path: Path::Scale, interp: Interp::Step, times, values }
}

enum Bullet {
    /// Alive frames `(turn, x, y)` from the snapshots.
    Tracked(Vec<(usize, f64, f64)>),
    /// Fired and hit on `turn`, never in a snapshot: from `from` to `to` (px).
    Tracer { turn: usize, from: (f64, f64), to: (f64, f64) },
}

pub(crate) fn add_bullets(scene: &mut Scene, replay: &Replay, space: &Space) {
    let d = bullet_diameter(replay, space);
    // Bullets fly at their shooter's muzzle height (the first robot's when the shooter is unknown).
    let muzzle: BTreeMap<usize, f64> = models(replay, space)
        .into_iter()
        .map(|(id, (_, m))| (id, f64::from(m.muzzle_height)))
        .collect();
    // The shooter from the snapshots' owner_id, then from bullet_fired events.
    let mut shooter: BTreeMap<u64, usize> = replay
        .turns
        .iter()
        .flat_map(|t| &t.bullets)
        .filter_map(|b| b.owner.map(|o| (b.id, o)))
        .collect();
    shooter.extend(
        replay
            .turns
            .iter()
            .flat_map(|t| &t.events.fired)
            .map(|f| (f.bullet_id, f.shooter)),
    );
    let fallback = replay
        .robots
        .first()
        .and_then(|r| muzzle.get(&r.id))
        .copied()
        .unwrap_or(0.0);
    let up = |bullet: u64| {
        shooter
            .get(&bullet)
            .and_then(|s| muzzle.get(s))
            .copied()
            .unwrap_or(fallback)
    };
    let turns = replay.turns.len();
    let mut bullets: BTreeMap<u64, Bullet> = BTreeMap::new();
    let mut seen = std::collections::BTreeSet::new();
    for (k, t) in replay.turns.iter().enumerate() {
        for b in &t.bullets {
            seen.insert(b.id);
            if b.alive {
                match bullets
                    .entry(b.id)
                    .or_insert_with(|| Bullet::Tracked(Vec::new()))
                {
                    Bullet::Tracked(frames) => frames.push((k, b.x, b.y)),
                    Bullet::Tracer { .. } => {}
                }
            }
        }
    }
    let robot_poses: BTreeMap<usize, Vec<RobotFrame>> =
        replay.robots.iter().map(|r| (r.id, poses(replay, r.id))).collect();
    let at = |id: usize, k: usize| {
        robot_poses.get(&id).and_then(|p| p.get(k)).map(|p| (p.x, p.y))
    };
    for (k, t) in replay.turns.iter().enumerate() {
        for f in &t.events.fired {
            if seen.contains(&f.bullet_id) {
                continue;
            }
            let Some(from) = at(f.shooter, k) else { continue };
            let to = t
                .events
                .hits
                .iter()
                .find(|h| h.bullet_id == f.bullet_id)
                .and_then(|h| at(h.victim, k))
                .or_else(|| {
                    t.events
                        .obstacle_hits
                        .iter()
                        .find(|o| o.bullet_id == f.bullet_id)
                        .map(|o| (o.x, o.y))
                });
            if let Some(to) = to {
                bullets
                    .insert(f.bullet_id, Bullet::Tracer { turn: k, from, to });
            }
        }
    }

    let mut sphere: Option<MeshRef> = None;
    for (id, bullet) in bullets {
        let colour = scene.material("bullet", BULLET_COLOUR);
        let mut node = Node::new(format!("bullet{id}"));
        match bullet {
            Bullet::Tracked(frames) => {
                let Some(&(first, ..)) = frames.first() else { continue };
                let last = frames.last().map_or(first, |f| f.0);
                // Built at its radius (and shared), so its texture coordinates are in metres.
                let sphere = *sphere.get_or_insert_with(|| {
                    scene.own(geometry::sphere(0.08 * d))
                });
                node.mesh = vec![(sphere, colour)];
                let n = scene.add(node, None);
                let translation = Track {
                    node: n,
                    path: Path::Translation,
                    interp: Interp::Linear,
                    times: frames.iter().map(|f| turn_time(f.0)).collect(),
                    values: frames
                        .iter()
                        .flat_map(|f| space.point(f.1, f.2, up(id)))
                        .collect(),
                };
                let scale = visibility(n, first, last, turns);
                scene.nodes[n].translation = first_key(&translation);
                scene.nodes[n].scale = first_key(&scale);
                scene.tracks.extend([translation, scale]);
            }
            Bullet::Tracer { turn, from, to } => {
                let (dx, dy) = (to.0 - from.0, to.1 - from.1);
                let w = 0.05 * d;
                let length = space.len(dx.hypot(dy)) as f32;
                node.mesh = vec![(scene.sized_box([w, w, length]), colour)];
                node.translation = space.point(
                    (from.0 + to.0) / 2.0,
                    (from.1 + to.1) / 2.0,
                    up(id),
                );
                node.rotation = yaw_toward(dx, dy);
                let n = scene.add(node, None);
                let scale = visibility(n, turn, turn, turns);
                scene.nodes[n].scale = first_key(&scale);
                scene.tracks.push(scale);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::{Interp, MeshRef, Path, build};
    use crate::glb::replay::{BulletFrame, Fired, Hit, ObstacleHit};
    use crate::glb::space::{Space, rotate, turn_time, yaw, yaw_toward};
    use crate::glb::tanks;

    const D: f32 = 9.0;

    fn close(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
    }

    fn key<const N: usize>(values: &[f32], k: usize) -> [f32; N] {
        values[k * N..(k + 1) * N].try_into().unwrap()
    }

    fn two(turns: usize) -> Vec<crate::glb::replay::Turn> {
        (0..turns)
            .map(|k| {
                turn(vec![
                    frame(0, 100.0 + k as f64, 100.0, 0.0),
                    frame(1, 700.0, 500.0, 0.0),
                ])
            })
            .collect()
    }

    #[test]
    fn names_are_stable() {
        let s = build(&replay(two(2)));
        for name in [
            "robot0",
            "robot0/hull",
            "robot0/gun",
            "robot0/gun/barrel",
            "robot0/radar",
            "robot1",
        ] {
            find(&s, name);
        }
    }

    #[test]
    fn robot_track_follows_recording() {
        let s = build(&replay(two(3)));
        let t = track(&s, find(&s, "robot0"), Path::Translation);
        assert_eq!(t.times, vec![0.0, turn_time(1), turn_time(2)]);
        assert_eq!(t.interp, Interp::Linear);
        assert_eq!(
            key::<3>(&t.values, 1),
            Space::new(None).point(101.0, 100.0, 0.0)
        );
        assert_eq!(
            s.nodes[find(&s, "robot0")].translation,
            Space::new(None).point(100.0, 100.0, 0.0)
        );
    }

    fn materials(s: &crate::glb::scene::Scene, node: &str) -> Vec<String> {
        s.nodes[find(s, node)]
            .mesh
            .iter()
            .map(|&(_, m)| s.materials[m].0.clone())
            .collect()
    }

    #[test]
    fn robots_drive_their_tanks() {
        let s = build(&replay(two(1)));
        let extras = |n: &str| s.nodes[find(&s, n)].extras.clone().unwrap();
        assert_eq!(extras("robot0")["tank"], "leopard2a8");
        assert_eq!(extras("robot1")["tank"], "abrams");
        assert!(
            materials(&s, "robot0/hull")
                .contains(&"leopard2a8/base".to_string())
        );
        assert!(
            materials(&s, "robot1/hull").contains(&"abrams/base".to_string())
        );
        // The team colour is on the muzzle cap and the dish only.
        assert!(
            materials(&s, "robot0/gun/barrel").contains(&"robot0".to_string())
        );
        assert!(materials(&s, "robot1/radar").contains(&"robot1".to_string()));
        assert!(!materials(&s, "robot0/hull").contains(&"robot0".to_string()));
        let model = tanks::build(&tanks::TANKS[0], 0.25);
        assert_eq!(s.nodes[find(&s, "robot0/gun")].translation, model.gun_at);
        assert_eq!(
            s.nodes[find(&s, "robot0/radar")].translation,
            model.radar_at
        );
        // No node is scaled: tanks are built at size.
        assert!(
            s.nodes
                .iter()
                .filter(|n| n.name.starts_with("robot"))
                .all(|n| n.scale == [1.0; 3])
        );

        let mut r = replay(two(1));
        r.tanks = Some(vec!["k2".into()]);
        let s = build(&r);
        assert_eq!(
            s.nodes[find(&s, "robot0")].extras.as_ref().unwrap()["tank"],
            "k2"
        );
    }

    #[test]
    fn gun_rotation_is_relative() {
        let mut f = frame(0, 100.0, 100.0, 90.0);
        f.gun_heading = 120.0;
        f.radar_heading = 60.0;
        let s =
            build(&replay(vec![turn(vec![f, frame(1, 700.0, 500.0, 0.0)])]));
        let gun = track(&s, find(&s, "robot0/gun"), Path::Rotation);
        assert!(close(&gun.values, &yaw(30.0)));
        let radar = track(&s, find(&s, "robot0/radar"), Path::Rotation);
        assert!(close(&radar.values, &yaw(-30.0)));
        let body = track(&s, find(&s, "robot0"), Path::Rotation);
        assert!(close(&body.values, &yaw(90.0)));
    }

    #[test]
    fn rotation_keys_stay_continuous() {
        let turns = [359.0, 1.0, 3.0]
            .iter()
            .map(|&h| {
                turn(vec![
                    frame(0, 100.0, 100.0, h),
                    frame(1, 700.0, 500.0, 0.0),
                ])
            })
            .collect();
        let s = build(&replay(turns));
        let t = track(&s, find(&s, "robot0"), Path::Rotation);
        for k in 1..3 {
            let (a, b) = (key::<4>(&t.values, k - 1), key::<4>(&t.values, k));
            assert!(a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>() > 0.0);
        }
    }

    #[test]
    fn dead_robot_holds_last_pose() {
        let mut turns = Vec::new();
        for k in 0..5 {
            let mut robots = vec![frame(0, 100.0, 100.0, 0.0)];
            if k < 3 {
                let mut f =
                    frame(1, 700.0 - 10.0 * k as f64, 500.0, 10.0 * k as f64);
                f.alive = k < 2;
                robots.push(f);
            }
            if k == 4 {
                // A stray frame after death must not move the wreck.
                let mut f = frame(1, 1.0, 1.0, 180.0);
                f.alive = false;
                robots.push(f);
            }
            turns.push(turn(robots));
        }
        let s = build(&replay(turns));
        let n = find(&s, "robot1");
        let (t, r) =
            (track(&s, n, Path::Translation), track(&s, n, Path::Rotation));
        for k in 2..5 {
            assert_eq!(
                key::<3>(&t.values, k),
                Space::new(None).point(680.0, 500.0, 0.0)
            );
            assert!(close(&key::<4>(&r.values, k), &yaw(20.0)));
        }
        // The wreck is squashed from the turn it dies (turn 2).
        let sc = track(&s, n, Path::Scale);
        assert_eq!(sc.interp, Interp::Step);
        assert_eq!(sc.times, vec![0.0, turn_time(2)]);
        assert!(close(&sc.values, &[1.0, 1.0, 1.0, 1.0, 0.4, 1.0]));
        // A robot that never dies has no scale track.
        let alive = find(&s, "robot0");
        assert!(
            s.tracks
                .iter()
                .all(|t| !(t.node == alive && t.path == Path::Scale))
        );
    }

    fn with_bullet(
        turns: usize,
        alive_on: &[usize],
        id: u64,
    ) -> Vec<crate::glb::replay::Turn> {
        let mut ts = two(turns);
        for &k in alive_on {
            ts[k].bullets.push(BulletFrame {
                id,
                x: 200.0 + k as f64,
                y: 100.0,
                alive: true,
                owner: None,
            });
        }
        ts
    }

    /// `(size, is shared)` of a node's single primitive: built at size, so the node is unscaled.
    fn mesh_size(s: &crate::glb::scene::Scene, n: usize) -> [f32; 3] {
        let [(MeshRef(m), _)] = s.nodes[n].mesh[..] else {
            panic!("{} is not one own mesh", s.nodes[n].name)
        };
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in &s.meshes[m].positions {
            for c in 0..3 {
                lo[c] = lo[c].min(p[c]);
                hi[c] = hi[c].max(p[c]);
            }
        }
        [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
    }

    #[test]
    fn tracked_bullet_scale_keys() {
        let s = build(&replay(with_bullet(5, &[2], 7)));
        let n = find(&s, "bullet7");
        // A sphere built at its radius, so its texture coordinates are in metres.
        let d = 2.0 * 0.08 * D;
        assert!(close(&mesh_size(&s, n), &[d, d, d]));
        let sc = track(&s, n, Path::Scale);
        assert_eq!(sc.interp, Interp::Step);
        assert_eq!(sc.times, vec![0.0, turn_time(2), turn_time(3)]);
        assert!(close(
            &sc.values,
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]
        ));
        let tr = track(&s, n, Path::Translation);
        assert_eq!(tr.times, vec![turn_time(2)]);
        let muzzle = tanks::build(&tanks::TANKS[0], 0.25).muzzle_height;
        assert!(close(
            &tr.values,
            &Space::new(None).point(202.0, 100.0, muzzle as f64)
        ));
        assert_eq!(s.nodes[n].scale, [0.0; 3]);
    }

    #[test]
    fn bullet_alive_at_end_keeps_scale() {
        let s = build(&replay(with_bullet(5, &[3, 4], 2)));
        let sc = track(&s, find(&s, "bullet2"), Path::Scale);
        assert_eq!(sc.times, vec![0.0, turn_time(3)]);
        assert!(sc.times.iter().all(|&t| t <= turn_time(4)));
    }

    #[test]
    fn bullet_alive_false_counts_as_gone() {
        let mut ts = with_bullet(5, &[1, 2], 4);
        ts[3].bullets.push(BulletFrame {
            id: 4,
            x: 0.0,
            y: 0.0,
            alive: false,
            owner: None,
        });
        let s = build(&replay(ts));
        let sc = track(&s, find(&s, "bullet4"), Path::Scale);
        assert_eq!(sc.times, vec![0.0, turn_time(1), turn_time(3)]);
    }

    #[test]
    fn same_turn_hit_becomes_tracer() {
        let mut ts = two(5);
        ts[2].events.fired.push(Fired { shooter: 0, bullet_id: 9 });
        ts[2].events.hits.push(Hit { victim: 1, bullet_id: 9 });
        let s = build(&replay(ts));
        let n = find(&s, "bullet9");
        let node = &s.nodes[n];
        let sp = Space::new(None);
        let (from, to) = ((102.0, 100.0), (700.0, 500.0));
        let muzzle = tanks::build(&tanks::TANKS[0], 0.25).muzzle_height;
        let mid = sp.point(
            (from.0 + to.0) / 2.0,
            (from.1 + to.1) / 2.0,
            muzzle as f64,
        );
        assert!(close(&node.translation, &mid));
        assert!(close(
            &node.rotation,
            &yaw_toward(to.0 - from.0, to.1 - from.1)
        ));
        let dist = sp.len((to.0 - from.0).hypot(to.1 - from.1)) as f32;
        let sc = track(&s, n, Path::Scale);
        assert_eq!(sc.interp, Interp::Step);
        assert_eq!(sc.times, vec![0.0, turn_time(2), turn_time(3)]);
        let w = 0.05 * D;
        assert!(close(
            &sc.values,
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]
        ));
        assert!(close(&mesh_size(&s, n), &[w, w, dist]));
        // The tracer's long axis points from shooter to victim.
        let dir = rotate(node.rotation, [0.0, 0.0, -1.0]);
        let want = sp.point(to.0 - from.0, to.1 - from.1, 0.0);
        let len = (want[0] * want[0] + want[2] * want[2]).sqrt();
        assert!(close(&dir, &[want[0] / len, 0.0, want[2] / len]));
    }

    #[test]
    fn obstacle_hit_tracer_ends_at_event_position() {
        let mut ts = two(3);
        ts[1].events.fired.push(Fired { shooter: 0, bullet_id: 5 });
        ts[1].events.obstacle_hits.push(ObstacleHit {
            bullet_id: 5,
            x: 301.0,
            y: 40.0,
        });
        let s = build(&replay(ts));
        let node = &s.nodes[find(&s, "bullet5")];
        let muzzle = tanks::build(&tanks::TANKS[0], 0.25).muzzle_height;
        let mid = Space::new(None).point(
            (101.0 + 301.0) / 2.0,
            (100.0 + 40.0) / 2.0,
            muzzle as f64,
        );
        assert!(close(&node.translation, &mid));
    }

    #[test]
    fn fired_without_impact_gets_no_node() {
        let mut ts = two(3);
        ts[1].events.fired.push(Fired { shooter: 0, bullet_id: 5 });
        let s = build(&replay(ts));
        assert!(s.nodes.iter().all(|n| n.name != "bullet5"));
    }

    #[test]
    fn ginza_bullets() {
        let r = crate::glb::replay::parse(include_str!(
            "../../../tests/fixtures/ginzabattle02.json"
        ))
        .unwrap();
        let s = build(&r);
        let bullets =
            s.nodes.iter().filter(|n| n.name.starts_with("bullet")).count();
        assert_eq!(bullets, 40);
        // Tracked spheres are the bullets that move.
        let spheres = s
            .tracks
            .iter()
            .filter(|t| {
                t.path == Path::Translation
                    && s.nodes[t.node].name.starts_with("bullet")
            })
            .count();
        assert_eq!(spheres, 19);
    }

    #[test]
    fn bullet_height_from_snapshot_owner() {
        // No bullet_fired event: the shooter comes from the snapshot's owner_id.
        let mut ts = with_bullet(3, &[1], 6);
        ts[1].bullets[0].owner = Some(1);
        let s = build(&replay(ts));
        let tr = track(&s, find(&s, "bullet6"), Path::Translation);
        let abrams = tanks::build(&tanks::TANKS[1], 0.25).muzzle_height;
        assert!((tr.values[1] - abrams).abs() < 1e-5, "{}", tr.values[1]);
    }

    #[test]
    fn same_tank_shares_geometry() {
        let mut r = replay(two(1));
        r.tanks = Some(vec!["k2".into(), "k2".into()]);
        let s = build(&r);
        let geom = |n: &str| -> Vec<MeshRef> {
            s.nodes[find(&s, n)].mesh.iter().map(|&(g, _)| g).collect()
        };
        for part in ["hull", "gun", "gun/barrel", "radar"] {
            assert_eq!(
                geom(&format!("robot0/{part}")),
                geom(&format!("robot1/{part}")),
                "{part}"
            );
        }
        // Only the team colour differs.
        let mats = |n: &str| -> Vec<usize> {
            s.nodes[find(&s, n)].mesh.iter().map(|&(_, m)| m).collect()
        };
        assert_ne!(mats("robot0/gun/barrel"), mats("robot1/gun/barrel"));
        assert_eq!(mats("robot0/hull"), mats("robot1/hull"));
    }
}
