//! The five tanks as low-poly meshes, ported from the viewer's `web/src/tanks/models3d.ts`
//! (sizes from `web/src/tanks/defs.ts`). Built in the viewer's "file" frame (x forward,
//! y up, z to the tank's right, in file units) and mapped to the robot's glTF frame
//! (forward −Z, up +Y, right +X, metres). Tracks and wheels are static.

use super::geometry::{Mesh, planar_uv};

/// What a part is painted with; `Base`, `Armor` and `Secondary` are the tank's own colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Paint {
    Base,
    Armor,
    Secondary,
    Track,
    Tread,
    Wheel,
    Hub,
    Glass,
    Dark,
    Lamp,
    /// The robot's slot colour (muzzle cap and radar dish).
    Team,
}

impl Paint {
    /// `(material name, sRGB hex)` for a tank; `Team` is handled by the caller.
    pub fn material(self, tank: &Tank) -> (String, String) {
        let own = |name: &str, c: u32| {
            (format!("{}/{name}", tank.id), format!("#{c:06x}"))
        };
        let shared =
            |name: &str, c: u32| (name.to_string(), format!("#{c:06x}"));
        match self {
            Paint::Base => own("base", tank.colors[0]),
            Paint::Secondary => own("secondary", tank.colors[1]),
            Paint::Armor => own("armor", tank.colors[2]),
            Paint::Track => shared("track", 0x222522),
            Paint::Tread => shared("tread", 0x454844),
            Paint::Wheel => shared("wheel", 0x30342f),
            Paint::Hub => shared("hub", 0x858575),
            Paint::Glass => shared("glass", 0x263e42),
            Paint::Dark => shared("dark", 0x242823),
            Paint::Lamp => shared("lamp", 0xd4bd80),
            Paint::Team => {
                unreachable!("the team colour is the robot's slot material")
            }
        }
    }
}

/// One tank's definition (`defs.ts`) and look (`models3d.ts` `LOOKS`).
#[derive(Debug, Clone, PartialEq)]
pub struct Tank {
    pub id: &'static str,
    /// Hull length in px, tracks included.
    hull_length: f64,
    /// Turret pivot offset from the robot centre along the hull in px, positive forward.
    pivot_y: f64,
    /// Muzzle distance from the robot centre in px.
    barrel_reach: f64,
    /// px per art pixel, and the hull sprite's width and pivot (art pixels).
    scale: f64,
    art_w: f64,
    art_pivot_x: f64,
    /// `[base, secondary, armor]`.
    colors: [u32; 3],
    length: f64,
    hull_height: f64,
    turret_length: f64,
    turret_width: f64,
    turret_height: f64,
    wheels: usize,
    track_radius: f64,
    angular: bool,
    armor_blocks: bool,
}

/// Hull width in px (tracks included), the same for every tank.
pub const HULL_WIDTH_PX: f64 = 28.0;
const PAD_COUNT: usize = 40;

pub const TANKS: [Tank; 5] = [
    Tank {
        id: "leopard2a8",
        hull_length: 46.4,
        pivot_y: -0.1,
        barrel_reach: 28.3,
        scale: 0.1393,
        art_w: 201.0,
        art_pivot_x: 97.5,
        colors: [0x77775b, 0x535746, 0x96947b],
        length: 4.6,
        hull_height: 0.85,
        turret_length: 2.15,
        turret_width: 1.95,
        turret_height: 0.68,
        wheels: 7,
        track_radius: 0.57,
        angular: true,
        armor_blocks: true,
    },
    Tank {
        id: "abrams",
        hull_length: 47.0,
        pivot_y: 0.4,
        barrel_reach: 29.1,
        scale: 0.1407,
        art_w: 199.0,
        art_pivot_x: 92.5,
        colors: [0x777354, 0x4f5140, 0x999477],
        length: 4.8,
        hull_height: 0.9,
        turret_length: 2.5,
        turret_width: 2.05,
        turret_height: 0.78,
        wheels: 7,
        track_radius: 0.58,
        angular: false,
        armor_blocks: true,
    },
    Tank {
        id: "k2",
        hull_length: 45.8,
        pivot_y: 0.2,
        barrel_reach: 28.9,
        scale: 0.1379,
        art_w: 203.0,
        art_pivot_x: 101.5,
        colors: [0x626b4c, 0x454c3c, 0x89916a],
        length: 4.5,
        hull_height: 0.77,
        turret_length: 2.1,
        turret_width: 1.85,
        turret_height: 0.57,
        wheels: 6,
        track_radius: 0.55,
        angular: true,
        armor_blocks: false,
    },
    Tank {
        id: "leclerc",
        hull_length: 45.1,
        pivot_y: 0.3,
        barrel_reach: 28.8,
        scale: 0.1366,
        art_w: 205.0,
        art_pivot_x: 102.5,
        colors: [0x777a5b, 0x4d5540, 0x989879],
        length: 4.45,
        hull_height: 0.76,
        turret_length: 2.0,
        turret_width: 1.85,
        turret_height: 0.59,
        wheels: 6,
        track_radius: 0.53,
        angular: true,
        armor_blocks: true,
    },
    Tank {
        id: "merkava",
        hull_length: 46.8,
        pivot_y: 0.4,
        barrel_reach: 29.1,
        scale: 0.14,
        art_w: 200.0,
        art_pivot_x: 99.5,
        colors: [0x9b9270, 0x625d4c, 0xb1a587],
        length: 5.0,
        hull_height: 0.98,
        turret_length: 2.0,
        turret_width: 1.85,
        turret_height: 0.66,
        wheels: 6,
        track_radius: 0.57,
        angular: false,
        armor_blocks: true,
    },
];

/// The tank robot `slot` drives: `tanks[slot]` when it names a tank, else the slot default.
pub fn tank_for(tanks: &[String], slot: usize) -> &'static Tank {
    tanks
        .get(slot)
        .and_then(|id| TANKS.iter().find(|t| t.id == id))
        .unwrap_or(&TANKS[slot % TANKS.len()])
}

/// A tank's meshes, each part in its node's frame, in metres.
#[derive(Debug, Clone, PartialEq)]
pub struct TankModel {
    /// Hull, skirts and tracks, in the robot's frame.
    pub hull: Vec<(Mesh, Paint)>,
    /// The turret, in the gun node's frame; `gun_at` is that node's place in the robot frame.
    pub turret: Vec<(Mesh, Paint)>,
    pub gun_at: [f32; 3],
    /// Barrel and muzzle cap, in the gun node's frame.
    pub barrel: Vec<(Mesh, Paint)>,
    /// Mast and dish, in the radar node's frame; `radar_at` is that node's place.
    pub radar: Vec<(Mesh, Paint)>,
    pub radar_at: [f32; 3],
    /// Height of the barrel's axis above the ground, in metres.
    pub muzzle_height: f32,
}

type M3 = [[f64; 3]; 3];
const IDENTITY: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

fn rot_x(a: f64) -> M3 {
    let (s, c) = a.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
}

fn rot_z(a: f64) -> M3 {
    let (s, c) = a.sin_cos();
    [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
}

fn apply(m: &M3, p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m[r][0] * p[0] + m[r][1] * p[1] + m[r][2] * p[2])
}

/// Primitives in the file frame, centred on their own origin (as three.js builds them).
mod shape {
    use super::super::geometry::{Mesh, cuboid};

    pub fn cube(size: [f64; 3]) -> Mesh {
        cuboid(size.map(|v| v as f32))
    }

    fn norm(v: [f32; 3]) -> [f32; 3] {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|c| c / l)
    }

    /// three.js `CylinderGeometry(top, bottom, len, sides)`: axis +y, flat-shaded, capped.
    pub fn cylinder(top: f64, bottom: f64, len: f64, sides: usize) -> Mesh {
        let mut m = Mesh::default();
        let h = (len / 2.0) as f32;
        let ring = |r: f64, y: f32| -> Vec<[f32; 3]> {
            (0..sides)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / sides as f64;
                    [(r * a.sin()) as f32, y, (r * a.cos()) as f32]
                })
                .collect()
        };
        let (t, b) = (ring(top, h), ring(bottom, -h));
        for i in 0..sides {
            let j = (i + 1) % sides;
            let mid = (std::f64::consts::TAU * (i as f64 + 0.5) / sides as f64)
                as f32;
            // Outward, tilted by the taper.
            let slope = ((bottom - top) / len) as f32;
            let n = norm([mid.sin(), slope, mid.cos()]);
            m.quad([b[i], b[j], t[j], t[i]], n);
        }
        for (r, y, ny) in [(&t, h, 1.0f32), (&b, -h, -1.0)] {
            let n = [0.0, ny, 0.0];
            let c = m.vertex([0.0, y, 0.0], n);
            let idx: Vec<u32> = r.iter().map(|&p| m.vertex(p, n)).collect();
            for i in 0..sides {
                m.triangle(c, idx[i], idx[(i + 1) % sides], n);
            }
        }
        m
    }

    /// A rounded-rectangle track belt in the x/y plane, `straight` long between the arc
    /// centres, from radius `inner` to `outer`, extruded `depth` along +z from z 0.
    pub fn belt(straight: f64, inner: f64, outer: f64, depth: f64) -> Mesh {
        const ARC: usize = 8;
        let half = straight / 2.0;
        // The loop's centres and outward directions, counter-clockwise from the bottom left.
        let mut path: Vec<([f64; 2], [f64; 2])> = Vec::new();
        for (cx, a0) in [
            (half, -std::f64::consts::FRAC_PI_2),
            (-half, std::f64::consts::FRAC_PI_2),
        ] {
            for k in 0..=ARC {
                let a = a0 + std::f64::consts::PI * k as f64 / ARC as f64;
                path.push(([cx, 0.0], [a.cos(), a.sin()]));
            }
        }
        let at = |(c, d): ([f64; 2], [f64; 2]), r: f64, z: f64| -> [f32; 3] {
            [(c[0] + d[0] * r) as f32, (c[1] + d[1] * r) as f32, z as f32]
        };
        let mut m = Mesh::default();
        let n = path.len();
        for i in 0..n {
            let (p, q) = (path[i], path[(i + 1) % n]);
            for (r, out) in [(outer, 1.0f32), (inner, -1.0)] {
                let (a, b) = (at(p, r, 0.0), at(q, r, 0.0));
                let (c, d) = (at(q, r, depth), at(p, r, depth));
                // Face normal: perpendicular to the edge in x/y, pointing out of (or into) the loop.
                let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
                let len = ex.hypot(ey);
                if len < 1e-9 {
                    continue;
                }
                let nrm = [out * ey / len, -out * ex / len, 0.0];
                m.quad([a, b, c, d], nrm);
            }
            for (z, nz) in [(0.0, -1.0f32), (depth, 1.0)] {
                let corners = [
                    at(p, inner, z),
                    at(q, inner, z),
                    at(q, outer, z),
                    at(p, outer, z),
                ];
                m.quad(corners, [0.0, 0.0, nz]);
            }
        }
        m
    }
}

/// Collects parts by paint, mapping file-frame points into a node frame in metres.
struct Group {
    parts: Vec<(Mesh, Paint)>,
    /// Metres per file unit.
    k: f64,
    /// Added after mapping (metres, node frame).
    offset: [f64; 3],
}

impl Group {
    fn new(k: f64, offset: [f64; 3]) -> Group {
        Group { parts: Vec::new(), k, offset }
    }

    /// File frame (x forward, y up, z right) → glTF robot frame (−Z forward, +Y up, +X right).
    fn map(p: [f64; 3]) -> [f64; 3] {
        [p[2], p[1], -p[0]]
    }

    fn add(&mut self, mesh: &Mesh, rot: M3, pos: [f64; 3], paint: Paint) {
        let mut out = Mesh::default();
        for (p, n) in mesh.positions.iter().zip(&mesh.normals) {
            let p = apply(&rot, p.map(f64::from));
            let q = Self::map([p[0] + pos[0], p[1] + pos[1], p[2] + pos[2]]);
            let q = [0, 1, 2].map(|c| (q[c] * self.k + self.offset[c]) as f32);
            let nn =
                Self::map(apply(&rot, n.map(f64::from))).map(|c| c as f32);
            out.positions.push(q);
            out.normals.push(nn);
            out.uvs.push(planar_uv(q, nn));
        }
        out.indices = mesh.indices.clone();
        match self.parts.iter_mut().find(|(_, p)| *p == paint) {
            Some((m, _)) => m.append(&out),
            None => self.parts.push((out, paint)),
        }
    }

    fn cube(&mut self, size: [f64; 3], pos: [f64; 3], paint: Paint) {
        self.add(&shape::cube(size), IDENTITY, pos, paint);
    }

    fn cube_rot(
        &mut self,
        size: [f64; 3],
        rot_z_by: f64,
        pos: [f64; 3],
        paint: Paint,
    ) {
        self.add(&shape::cube(size), rot_z(rot_z_by), pos, paint);
    }

    /// The viewer's `cyl`: axis along the width (z) by default, or the length (x).
    fn cyl(
        &mut self,
        r: f64,
        len: f64,
        pos: [f64; 3],
        paint: Paint,
        sides: usize,
        along_x: bool,
    ) {
        let rot = if along_x {
            rot_z(std::f64::consts::FRAC_PI_2)
        } else {
            rot_x(std::f64::consts::FRAC_PI_2)
        };
        self.add(&shape::cylinder(r, r, len, sides), rot, pos, paint);
    }

    fn sorted(mut self) -> Vec<(Mesh, Paint)> {
        self.parts.sort_by_key(|(_, p)| *p);
        self.parts
    }
}

/// Point on the rounded-rectangle track loop in the x/y plane (`trackPoint`).
fn track_point(distance: f64, straight: f64, radius: f64) -> [f64; 2] {
    let arc = std::f64::consts::PI * radius;
    let total = 2.0 * straight + 2.0 * arc;
    let mut s = distance.rem_euclid(total);
    if s < straight {
        return [-straight / 2.0 + s, -radius];
    }
    s -= straight;
    if s < arc {
        let a = -std::f64::consts::FRAC_PI_2 + s / radius;
        return [straight / 2.0 + radius * a.cos(), radius * a.sin()];
    }
    s -= arc;
    if s < straight {
        return [straight / 2.0 - s, radius];
    }
    s -= straight;
    let a = std::f64::consts::FRAC_PI_2 + s / radius;
    [-straight / 2.0 + radius * a.cos(), radius * a.sin()]
}

/// Builds `tank` at `m_per_px` metres per battlefield px.
pub fn build(tank: &Tank, m_per_px: f64) -> TankModel {
    use Paint::*;
    let k_px = tank.hull_length / tank.length;
    let k = k_px * m_per_px;
    let width = HULL_WIDTH_PX / k_px - 0.32;
    let (len, h, r) = (tank.length, tank.hull_height, tank.track_radius);
    let pivot_x = tank.pivot_y / k_px;
    let ring_z = ((tank.art_pivot_x - tank.art_w / 2.0) * tank.scale) / k_px;
    let lift = h / 2.0 * k;

    // ---------------- hull ----------------
    let mut hull = Group::new(k, [0.0, lift, 0.0]);
    hull.cube([len, h, width], [0.0; 3], Base);
    hull.cube_rot(
        [0.9, h * 0.8, width * 0.96],
        -0.22,
        [len / 2.0 - 0.4, h * 0.12, 0.0],
        Armor,
    );
    hull.cube(
        [1.05, 0.22, width * 0.85],
        [-len / 2.0 + 0.48, h * 0.58, 0.0],
        Secondary,
    );
    for i in 0..5 {
        hull.cube(
            [0.06, 0.025, width * 0.65],
            [-len / 2.0 + 0.2 + i as f64 * 0.14, h * 0.72, 0.0],
            Dark,
        );
    }
    for side in [-1.0, 1.0] {
        hull.cube(
            [0.12, 0.13, 0.16],
            [len / 2.0 - 0.12, h * 0.1, side * width * 0.37],
            Lamp,
        );
        hull.cube(
            [len * 0.78, 0.58, 0.12],
            [-0.05, -0.08, side * (width / 2.0 + 0.1)],
            Armor,
        );
    }
    if tank.id == "merkava" {
        hull.cube(
            [1.5, 0.45, width * 0.92],
            [-len * 0.34, h * 0.73, 0.0],
            Armor,
        );
        hull.cube(
            [0.12, 0.48, width * 0.7],
            [-len / 2.0 - 0.04, 0.0, 0.0],
            Secondary,
        );
    }

    // ---------------- tracks ----------------
    let straight = len - r * 2.0;
    let perimeter = 2.0 * straight + 2.0 * std::f64::consts::PI * r;
    let track_y = -h / 2.0 + r + 0.09;
    let pad = shape::cube([(perimeter / PAD_COUNT as f64) * 0.88, 0.17, 0.15]);
    let belt = shape::belt(straight, r - 0.09, r + 0.09, 0.12);
    for side in [1.0, -1.0] {
        hull.add(
            &belt,
            IDENTITY,
            [0.0, track_y, side * (width / 2.0) - 0.06],
            Track,
        );
        for i in 0..PAD_COUNT {
            let s = (i as f64 / PAD_COUNT as f64) * perimeter;
            let [x, y] = track_point(s, straight, r);
            let [nx, ny] = track_point(s - 0.01, straight, r);
            let a = (ny - y).atan2(nx - x);
            hull.add(
                &pad,
                rot_z(a),
                [x, y + track_y, side * (width / 2.0 + 0.075)],
                Tread,
            );
        }
        for i in 0..tank.wheels {
            let x = -straight / 2.0
                + (i as f64 / (tank.wheels - 1) as f64) * straight;
            hull.cyl(
                r * 0.49,
                0.19,
                [x, track_y, side * (width / 2.0 + 0.035)],
                Wheel,
                12,
                false,
            );
            hull.cyl(
                r * 0.2,
                0.21,
                [x, track_y, side * (width / 2.0 + 0.04)],
                Hub,
                8,
                false,
            );
        }
    }

    // ---------------- turret ----------------
    let (tl, tw, th) =
        (tank.turret_length, tank.turret_width, tank.turret_height);
    let roof = 0.42 + th / 2.0;
    let gun_at = Group::map([pivot_x, h * 0.65, ring_z]).map(|c| c * k);
    let gun_at = [gun_at[0], gun_at[1] + lift, gun_at[2]];
    let mut t = Group::new(k, [0.0; 3]);
    t.cyl(tw * 0.43, 0.24, [0.0, 0.1, 0.0], Secondary, 8, false);
    t.cube([tl, th, tw], [0.0, 0.42, 0.0], Armor);
    t.cube_rot(
        [0.55, th * 0.9, tw * 0.9],
        -0.17,
        [tl * 0.36, 0.42, 0.0],
        Base,
    );
    t.cyl(0.25, 0.09, [-0.45, roof, 0.0], Secondary, 8, false);
    t.cyl(0.27, 0.45, [tl * 0.5, 0.48, 0.0], Secondary, 10, false);
    t.cube([0.24, 0.16, 0.3], [-0.48, roof + 0.22, 0.0], Glass);
    if tank.armor_blocks {
        for side in [-1.0, 1.0] {
            for i in 0..3 {
                t.cube(
                    [0.48, 0.38, 0.22],
                    [-0.45 + i as f64 * 0.48, 0.35, side * (tw / 2.0 + 0.08)],
                    Secondary,
                );
            }
        }
    }
    if tank.angular {
        t.cube([0.7, 0.3, 0.35], [-0.45, 0.76, 0.5], Base);
    }
    for p in [
        [0.55, 0.78, 0.78],
        [0.55, 0.78, -0.78],
        [-0.65, 0.78, 0.78],
        [-0.65, 0.78, -0.78],
    ] {
        t.cube([0.18, 0.14, 0.18], p, Glass);
    }
    for side in [-1.0, 1.0] {
        t.cube([0.25, 0.22, 0.3], [0.05, 0.75, side * 0.9], Dark);
    }
    t.cube([0.55, 0.08, 0.5], [-0.1, 0.83, -0.42], Secondary);
    t.add(
        &shape::cylinder(0.018, 0.018, 0.85, 5),
        IDENTITY,
        [-0.7, 1.1, -0.5],
        Dark,
    );

    // Barrel: its tip lands barrelReach from the robot centre.
    let gun_x = tl * 0.5;
    let barrel_length = tank.barrel_reach / k_px - pivot_x - gun_x;
    let mut barrel = Group::new(k, [0.0; 3]);
    barrel.add(
        &shape::cylinder(0.095, 0.14, barrel_length, 8),
        rot_z(-std::f64::consts::FRAC_PI_2),
        [gun_x + barrel_length / 2.0, 0.48, 0.0],
        Secondary,
    );
    barrel.cyl(
        0.15,
        0.24,
        [gun_x + barrel_length - 0.12, 0.48, 0.0],
        Team,
        8,
        true,
    );

    // Radar: mast and dish on the roof above the pivot.
    let mut radar = Group::new(k, [0.0; 3]);
    radar.add(
        &shape::cylinder(0.04, 0.04, 0.2, 5),
        IDENTITY,
        [0.0, 0.1, 0.0],
        Dark,
    );
    radar.add(
        &shape::cylinder(0.34, 0.06, 0.1, 8),
        IDENTITY,
        [0.0, 0.25, 0.0],
        Team,
    );
    let radar_at = [gun_at[0], gun_at[1] + roof * k, gun_at[2]];

    TankModel {
        hull: hull.sorted(),
        turret: t.sorted(),
        gun_at: gun_at.map(|c| c as f32),
        barrel: barrel.sorted(),
        radar: radar.sorted(),
        radar_at: radar_at.map(|c| c as f32),
        muzzle_height: (gun_at[1] + 0.48 * k) as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glb::geometry::triangle_normal;

    const M_PER_PX: f64 = 0.25;

    fn bounds(
        parts: &[(Mesh, Paint)],
        offset: [f32; 3],
    ) -> ([f32; 3], [f32; 3]) {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for (m, _) in parts {
            for p in &m.positions {
                for c in 0..3 {
                    lo[c] = lo[c].min(p[c] + offset[c]);
                    hi[c] = hi[c].max(p[c] + offset[c]);
                }
            }
        }
        (lo, hi)
    }

    #[test]
    fn footprint_matches_the_definitions() {
        for tank in &TANKS {
            let m = build(tank, M_PER_PX);
            let (lo, hi) = bounds(&m.hull, [0.0; 3]);
            let (w, l) = (hi[0] - lo[0], hi[2] - lo[2]);
            let (want_w, want_l) = (
                (HULL_WIDTH_PX * M_PER_PX) as f32,
                (tank.hull_length * M_PER_PX) as f32,
            );
            assert!(
                (w - want_w).abs() / want_w < 0.1,
                "{} width {w}",
                tank.id
            );
            assert!(
                (l - want_l).abs() / want_l < 0.1,
                "{} length {l}",
                tank.id
            );
            assert!(
                lo[1].abs() < 0.05,
                "{} sits on the ground: {}",
                tank.id,
                lo[1]
            );
        }
    }

    #[test]
    fn muzzle_tip_at_barrel_reach_and_pivot_at_ring() {
        for tank in &TANKS {
            let m = build(tank, M_PER_PX);
            // Forward is −Z: the tip is the barrel's most negative z, in the robot frame.
            let (lo, _) = bounds(&m.barrel, m.gun_at);
            let want = -(tank.barrel_reach * M_PER_PX) as f32;
            assert!(
                (lo[2] - want).abs() < 0.15,
                "{} tip {} want {want}",
                tank.id,
                lo[2]
            );
            assert!(
                (m.gun_at[2] + (tank.pivot_y * M_PER_PX) as f32).abs() < 1e-4
            );
            assert!(m.radar_at[1] > m.gun_at[1]);
            assert!(m.muzzle_height > m.gun_at[1]);
        }
    }

    #[test]
    fn team_colour_only_on_muzzle_and_dish() {
        let m = build(&TANKS[0], M_PER_PX);
        assert!(
            m.hull.iter().chain(&m.turret).all(|(_, p)| *p != Paint::Team)
        );
        assert!(m.barrel.iter().any(|(_, p)| *p == Paint::Team));
        assert!(m.radar.iter().any(|(_, p)| *p == Paint::Team));
    }

    #[test]
    fn meshes_are_well_formed() {
        for tank in &TANKS {
            let m = build(tank, M_PER_PX);
            for (mesh, _) in
                m.hull.iter().chain(&m.turret).chain(&m.barrel).chain(&m.radar)
            {
                assert_eq!(mesh.indices.len() % 3, 0);
                assert_eq!(mesh.uvs.len(), mesh.positions.len());
                assert!(
                    mesh.indices
                        .iter()
                        .all(|&i| (i as usize) < mesh.positions.len())
                );
                for n in &mesh.normals {
                    assert!(
                        (n[0] * n[0] + n[1] * n[1] + n[2] * n[2] - 1.0).abs()
                            < 1e-3
                    );
                }
                // Front faces are counter-clockwise: each triangle's winding agrees with its normal.
                for t in 0..mesh.indices.len() / 3 {
                    let n = triangle_normal(mesh, t);
                    if n[0] * n[0] + n[1] * n[1] + n[2] * n[2] < 1e-12 {
                        continue;
                    }
                    let s = mesh.normals[mesh.indices[3 * t] as usize];
                    assert!(
                        n[0] * s[0] + n[1] * s[1] + n[2] * s[2] > 0.0,
                        "{} triangle {t}",
                        tank.id
                    );
                }
            }
        }
    }

    #[test]
    fn cylinder_and_belt_face_out() {
        let c = shape::cylinder(0.3, 0.5, 2.0, 8);
        for t in 0..c.indices.len() / 3 {
            let n = triangle_normal(&c, t);
            let p = c.positions[c.indices[3 * t] as usize];
            let q = c.positions[c.indices[3 * t + 1] as usize];
            let r = c.positions[c.indices[3 * t + 2] as usize];
            let centre = [
                (p[0] + q[0] + r[0]) / 3.0,
                (p[1] + q[1] + r[1]) / 3.0,
                (p[2] + q[2] + r[2]) / 3.0,
            ];
            assert!(
                n[0] * centre[0] + n[1] * centre[1] + n[2] * centre[2] > 0.0,
                "triangle {t}"
            );
        }
        // Belt: winding agrees with the stored normals.
        let b = shape::belt(3.0, 0.4, 0.6, 0.2);
        for t in 0..b.indices.len() / 3 {
            let n = triangle_normal(&b, t);
            let stored = b.normals[b.indices[3 * t] as usize];
            assert!(
                n[0] * stored[0] + n[1] * stored[1] + n[2] * stored[2] > 0.0,
                "belt triangle {t}"
            );
        }
    }

    #[test]
    fn tank_for_uses_header_then_slot() {
        let tanks = vec!["k2".to_string(), "nope".to_string()];
        assert_eq!(tank_for(&tanks, 0).id, "k2");
        assert_eq!(tank_for(&tanks, 1).id, "abrams");
        assert_eq!(tank_for(&[], 2).id, "k2");
        assert_eq!(tank_for(&[], 6).id, "abrams");
    }
}
