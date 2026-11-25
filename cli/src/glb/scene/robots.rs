//! Robots (body, gun, radar), tracked bullets and one-turn tracers.

use std::collections::BTreeMap;

use serde_json::json;

use super::{Interp, MeshRef, Node, Path, Scene, Space, Track};
use crate::glb::replay::{Replay, RobotFrame};
use crate::glb::space::{continuous, turn_time, yaw, yaw_toward};

/// Slot colours, as in the viewer's `ROBOT_COLORS`.
const ROBOT_COLOURS: [&str; 4] = ["#e0a526", "#3fb6c9", "#ef5d79", "#9c7cf4"];
const BULLET_COLOUR: &str = "#fff6c8";
const DEFAULT_RADIUS_PX: f64 = 18.0;

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

pub(crate) fn add_robots(scene: &mut Scene, replay: &Replay, space: &Space) {
    for (slot, info) in replay.robots.iter().enumerate() {
        let poses = poses(replay, info.id);
        if poses.is_empty() {
            continue;
        }
        let d = diameter(replay, info.id, space);
        let colour = scene.material(
            &format!("robot{slot}"),
            ROBOT_COLOURS[slot % ROBOT_COLOURS.len()],
        );
        let name = format!("robot{}", info.id);

        let mut root = Node::new(&name);
        root.extras = Some(json!({ "name": info.name, "slot": slot }));
        let root = scene.add(root, None);

        let mut body = Node::new(format!("{name}/body"));
        body.mesh = Some((MeshRef::Box, colour));
        body.translation = [0.0, 0.14 * d, 0.0];
        body.scale = [d, 0.28 * d, d];
        scene.add(body, Some(root));

        let gun = scene.add(Node::new(format!("{name}/gun")), Some(root));
        let mut barrel = Node::new(format!("{name}/gun/barrel"));
        barrel.mesh = Some((MeshRef::Box, colour));
        barrel.translation = [0.0, 0.33 * d, -0.325 * d];
        barrel.scale = [0.1 * d, 0.1 * d, 0.65 * d];
        scene.add(barrel, Some(gun));

        let mut radar = Node::new(format!("{name}/radar"));
        radar.mesh = Some((MeshRef::Box, colour));
        radar.translation = [0.0, 0.4 * d, 0.0];
        radar.scale = [0.28 * d, 0.08 * d, 0.1 * d];
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
    }
}

/// A STEP scale track that shows `full` on turns `first..=last` and 0 otherwise.
fn visibility(
    node: usize,
    first: usize,
    last: usize,
    turns: usize,
    full: [f32; 3],
) -> Track {
    let mut times = Vec::new();
    let mut values = Vec::new();
    if first > 0 {
        times.push(0.0);
        values.extend([0.0; 3]);
    }
    times.push(turn_time(first));
    values.extend(full);
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
    let up = f64::from(0.33 * d);
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

    for (id, bullet) in bullets {
        let colour = scene.material("bullet", BULLET_COLOUR);
        let mut node = Node::new(format!("bullet{id}"));
        match bullet {
            Bullet::Tracked(frames) => {
                let Some(&(first, ..)) = frames.first() else { continue };
                let last = frames.last().map_or(first, |f| f.0);
                let r = 0.08 * d;
                node.mesh = Some((MeshRef::Sphere, colour));
                let n = scene.add(node, None);
                let translation = Track {
                    node: n,
                    path: Path::Translation,
                    interp: Interp::Linear,
                    times: frames.iter().map(|f| turn_time(f.0)).collect(),
                    values: frames
                        .iter()
                        .flat_map(|f| space.point(f.1, f.2, up))
                        .collect(),
                };
                let scale = visibility(n, first, last, turns, [r; 3]);
                scene.nodes[n].translation = first_key(&translation);
                scene.nodes[n].scale = first_key(&scale);
                scene.tracks.extend([translation, scale]);
            }
            Bullet::Tracer { turn, from, to } => {
                let (dx, dy) = (to.0 - from.0, to.1 - from.1);
                let w = 0.05 * d;
                node.mesh = Some((MeshRef::Box, colour));
                node.translation = space.point(
                    (from.0 + to.0) / 2.0,
                    (from.1 + to.1) / 2.0,
                    up,
                );
                node.rotation = yaw_toward(dx, dy);
                let n = scene.add(node, None);
                let scale = visibility(
                    n,
                    turn,
                    turn,
                    turns,
                    [w, w, space.len(dx.hypot(dy)) as f32],
                );
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
            "robot0/body",
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

    #[test]
    fn body_is_robot_diameter() {
        let s = build(&replay(two(1)));
        let body = &s.nodes[find(&s, "robot0/body")];
        assert!(close(&body.scale, &[9.0, 2.52, 9.0]));
        assert!(close(&body.translation, &[0.0, 0.14 * D, 0.0]));
        assert!(matches!(body.mesh, Some((MeshRef::Box, _))));
        let barrel = &s.nodes[find(&s, "robot0/gun/barrel")];
        assert!(close(&barrel.scale, &[0.1 * D, 0.1 * D, 0.65 * D]));
        assert!(close(&barrel.translation, &[0.0, 0.33 * D, -0.325 * D]));
        let radar = &s.nodes[find(&s, "robot0/radar")];
        assert!(close(&radar.scale, &[0.28 * D, 0.08 * D, 0.1 * D]));
        assert!(close(&radar.translation, &[0.0, 0.4 * D, 0.0]));
        let colour = |n: &str| s.nodes[find(&s, n)].mesh.unwrap().1;
        assert_ne!(colour("robot0/body"), colour("robot1/body"));
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
            });
        }
        ts
    }

    #[test]
    fn tracked_bullet_scale_keys() {
        let s = build(&replay(with_bullet(5, &[2], 7)));
        let n = find(&s, "bullet7");
        assert!(matches!(s.nodes[n].mesh, Some((MeshRef::Sphere, _))));
        let sc = track(&s, n, Path::Scale);
        assert_eq!(sc.interp, Interp::Step);
        assert_eq!(sc.times, vec![0.0, turn_time(2), turn_time(3)]);
        let full = 0.08 * D;
        assert!(close(
            &sc.values,
            &[0.0, 0.0, 0.0, full, full, full, 0.0, 0.0, 0.0]
        ));
        let tr = track(&s, n, Path::Translation);
        assert_eq!(tr.times, vec![turn_time(2)]);
        assert!(close(
            &tr.values,
            &Space::new(None).point(202.0, 100.0, (0.33 * D) as f64)
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
        let mid = sp.point(
            (from.0 + to.0) / 2.0,
            (from.1 + to.1) / 2.0,
            (0.33 * D) as f64,
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
            &[0.0, 0.0, 0.0, w, w, dist, 0.0, 0.0, 0.0]
        ));
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
        let mid = Space::new(None).point(
            (101.0 + 301.0) / 2.0,
            (100.0 + 40.0) / 2.0,
            (0.33 * D) as f64,
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
        let spheres = s
            .nodes
            .iter()
            .filter(|n| matches!(n.mesh, Some((MeshRef::Sphere, _))))
            .count();
        assert_eq!(spheres, 19);
    }
}
