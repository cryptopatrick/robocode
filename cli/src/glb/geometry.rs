//! Triangle meshes: the shared unit box and sphere, extruded footprints and road strips.
//!
//! Every triangle is wound counter-clockwise seen from outside, as glTF requires for front faces.

use super::space::Space;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl Mesh {
    fn vertex(&mut self, p: [f32; 3], n: [f32; 3]) -> u32 {
        self.positions.push(p);
        self.normals.push(n);
        (self.positions.len() - 1) as u32
    }

    /// Adds a triangle, wound so that its face points along `facing`.
    fn triangle(&mut self, a: u32, b: u32, c: u32, facing: [f32; 3]) {
        let (pa, pb, pc) = (
            self.positions[a as usize],
            self.positions[b as usize],
            self.positions[c as usize],
        );
        let n = cross(sub(pb, pa), sub(pc, pa));
        if dot(n, facing) < 0.0 {
            self.indices.extend([a, c, b]);
        } else {
            self.indices.extend([a, b, c]);
        }
    }

    /// Adds a flat quad (corners in order around it) facing along `normal`.
    fn quad(&mut self, corners: [[f32; 3]; 4], normal: [f32; 3]) {
        let i = corners.map(|p| self.vertex(p, normal));
        self.triangle(i[0], i[1], i[2], normal);
        self.triangle(i[0], i[2], i[3], normal);
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn signed_area(ring: &[[f64; 2]]) -> f64 {
    (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// A cube with 1 m edges, centred on the origin: 24 vertices, flat normals.
pub fn unit_box() -> Mesh {
    let mut m = Mesh::default();
    for axis in 0..3 {
        for sign in [-1.0f32, 1.0] {
            let mut n = [0.0; 3];
            n[axis] = sign;
            let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
            let corner = |su: f32, sv: f32| {
                let mut p = [0.0; 3];
                p[axis] = 0.5 * sign;
                p[u] = 0.5 * su;
                p[v] = 0.5 * sv;
                p
            };
            m.quad(
                [
                    corner(-1.0, -1.0),
                    corner(1.0, -1.0),
                    corner(1.0, 1.0),
                    corner(-1.0, 1.0),
                ],
                n,
            );
        }
    }
    m
}

/// A sphere of radius 1: 12 segments × 8 rings.
pub fn unit_sphere() -> Mesh {
    const SEGMENTS: usize = 12;
    const RINGS: usize = 8;
    let mut m = Mesh::default();
    for r in 0..=RINGS {
        let polar = std::f32::consts::PI * r as f32 / RINGS as f32;
        for s in 0..=SEGMENTS {
            let azimuth = std::f32::consts::TAU * s as f32 / SEGMENTS as f32;
            let p = [
                polar.sin() * azimuth.cos(),
                polar.cos(),
                polar.sin() * azimuth.sin(),
            ];
            m.vertex(p, p);
        }
    }
    for r in 0..RINGS {
        for s in 0..SEGMENTS {
            let a = (r * (SEGMENTS + 1) + s) as u32;
            let b = a + SEGMENTS as u32 + 1;
            for [i, j, k] in [[a, b, a + 1], [a + 1, b, b + 1]] {
                let centre = [0, 1, 2].map(|c| {
                    (m.positions[i as usize][c]
                        + m.positions[j as usize][c]
                        + m.positions[k as usize][c])
                        / 3.0
                });
                let n = cross(
                    sub(m.positions[j as usize], m.positions[i as usize]),
                    sub(m.positions[k as usize], m.positions[i as usize]),
                );
                if dot(n, n) < 1e-12 {
                    continue; // degenerate at a pole
                }
                m.triangle(i, j, k, centre);
            }
        }
    }
    m
}

/// Ear-clips an open ring (either winding) into `n − 2` triangles of indices into `ring`.
/// The triangles keep the ring's winding.
pub fn triangulate(ring: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let n = ring.len();
    if n < 3 {
        return Vec::new();
    }
    let sign = if signed_area(ring) < 0.0 { -1.0 } else { 1.0 };
    let turn = |a: usize, b: usize, c: usize| {
        let (a, b, c) = (ring[a], ring[b], ring[c]);
        sign * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]))
    };
    let mut left: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n - 2);
    while left.len() > 3 {
        let m = left.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) =
                (left[(i + m - 1) % m], left[i], left[(i + 1) % m]);
            if turn(a, b, c) <= 0.0 {
                return false;
            }
            left.iter().all(|&p| {
                p == a
                    || p == b
                    || p == c
                    || !(turn(a, b, p) >= 0.0
                        && turn(b, c, p) >= 0.0
                        && turn(c, a, p) >= 0.0)
            })
        });
        // A ring with only collinear or touching vertices left has no strict ear: clip any vertex.
        let i = ear.unwrap_or(0);
        out.push([left[(i + m - 1) % m], left[i], left[(i + 1) % m]]);
        left.remove(i);
    }
    out.push([left[0], left[1], left[2]]);
    out
}

/// A footprint in glTF `(X, Z)` metres extruded from y 0 to `height_m`: walls and roof, no floor.
pub fn extrude(ring_m: &[[f64; 2]], height_m: f64) -> Mesh {
    let mut m = Mesh::default();
    let h = height_m as f32;
    let sign = if signed_area(ring_m) < 0.0 { -1.0 } else { 1.0 };
    let n = ring_m.len();
    for i in 0..n {
        let (a, b) = (ring_m[i], ring_m[(i + 1) % n]);
        let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dz);
        if len == 0.0 {
            continue;
        }
        let normal =
            [(sign * dz / len) as f32, 0.0, (-sign * dx / len) as f32];
        let (ax, az, bx, bz) =
            (a[0] as f32, a[1] as f32, b[0] as f32, b[1] as f32);
        m.quad(
            [[ax, 0.0, az], [bx, 0.0, bz], [bx, h, bz], [ax, h, az]],
            normal,
        );
    }
    let up = [0.0, 1.0, 0.0];
    let roof: Vec<u32> = ring_m
        .iter()
        .map(|p| m.vertex([p[0] as f32, h, p[1] as f32], up))
        .collect();
    for [a, b, c] in triangulate(ring_m) {
        m.triangle(roof[a], roof[b], roof[c], up);
    }
    m
}

/// Clips the segment `a`–`b` to the rectangle `[0, w] × [0, h]` (Liang–Barsky).
pub fn clip_segment(
    a: [f64; 2],
    b: [f64; 2],
    w: f64,
    h: f64,
) -> Option<([f64; 2], [f64; 2])> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, a[0]), (dx, w - a[0]), (-dy, a[1]), (dy, h - a[1])] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    if t0 > t1 {
        return None;
    }
    let at = |t: f64| [a[0] + t * dx, a[1] + t * dy];
    Some((at(t0), at(t1)))
}

/// A flat strip along a centre line (px), clipped to the arena, with its top at `top_m`.
/// One quad per clipped segment; `None` when no segment enters the arena.
pub fn strip(
    points_px: &[[f64; 2]],
    width_px: f64,
    arena: (f64, f64),
    space: &Space,
    top_m: f64,
) -> Option<Mesh> {
    let mut m = Mesh::default();
    for pair in points_px.windows(2) {
        let Some((a, b)) = clip_segment(pair[0], pair[1], arena.0, arena.1)
        else {
            continue;
        };
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy);
        if len == 0.0 {
            continue;
        }
        let (ox, oy) = (-dy / len * width_px / 2.0, dx / len * width_px / 2.0);
        let corner = |p: [f64; 2], s: f64| {
            space.point(p[0] + s * ox, p[1] + s * oy, top_m)
        };
        m.quad(
            [corner(a, 1.0), corner(b, 1.0), corner(b, -1.0), corner(a, -1.0)],
            [0.0, 1.0, 0.0],
        );
    }
    (!m.indices.is_empty()).then_some(m)
}

#[cfg(test)]
pub(crate) fn triangle_normal(m: &Mesh, t: usize) -> [f32; 3] {
    let p = |i: usize| m.positions[m.indices[3 * t + i] as usize];
    let (a, b, c) = (p(0), p(1), p(2));
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shoelace(ring: &[[f64; 2]]) -> f64 {
        (0..ring.len())
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0
    }

    fn tri_area(ring: &[[f64; 2]], t: [usize; 3]) -> f64 {
        shoelace(&[ring[t[0]], ring[t[1]], ring[t[2]]]).abs()
    }

    fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn check_closed(m: &Mesh) {
        assert_eq!(m.indices.len() % 3, 0);
        assert_eq!(m.positions.len(), m.normals.len());
        assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
        for n in &m.normals {
            assert!((dot3(*n, *n) - 1.0).abs() < 1e-4);
        }
    }

    /// Every non-degenerate triangle faces away from `centre`.
    fn faces_out(m: &Mesh, centre: [f32; 3]) {
        for t in 0..m.indices.len() / 3 {
            let n = triangle_normal(m, t);
            if dot3(n, n) < 1e-10 {
                continue;
            }
            let p = m.positions[m.indices[3 * t] as usize];
            let out = [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]];
            assert!(dot3(n, out) > 0.0, "triangle {t} faces inward");
        }
    }

    #[test]
    fn box_and_sphere_are_closed() {
        let b = unit_box();
        check_closed(&b);
        assert_eq!((b.positions.len(), b.indices.len()), (24, 36));
        assert!(b.positions.iter().all(|p| p.iter().all(|c| c.abs() == 0.5)));
        faces_out(&b, [0.0; 3]);
        let s = unit_sphere();
        check_closed(&s);
        assert!(s.positions.iter().all(|p| (dot3(*p, *p) - 1.0).abs() < 1e-4));
        faces_out(&s, [0.0; 3]);
    }

    #[test]
    fn l_shape_triangulates() {
        let l = [
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 1.0],
            [1.0, 1.0],
            [1.0, 2.0],
            [0.0, 2.0],
        ];
        for ring in [l.to_vec(), l.iter().rev().copied().collect()] {
            let tris = triangulate(&ring);
            assert_eq!(tris.len(), 4);
            let area: f64 = tris.iter().map(|&t| tri_area(&ring, t)).sum();
            assert!((area - 3.0).abs() < 1e-9, "area {area}");
        }
    }

    #[test]
    fn roof_faces_up() {
        let square = [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        for ring in [square.to_vec(), square.iter().rev().copied().collect()] {
            let m = extrude(&ring, 10.0);
            check_closed(&m);
            let mut roofs = 0;
            for t in 0..m.indices.len() / 3 {
                let ys: Vec<f32> = (0..3)
                    .map(|i| m.positions[m.indices[3 * t + i] as usize][1])
                    .collect();
                if ys.iter().all(|&y| y == 10.0) {
                    roofs += 1;
                    assert!(triangle_normal(&m, t)[1] > 0.0);
                }
            }
            assert_eq!(roofs, 2);
            faces_out(&m, [2.0, 5.0, 2.0]);
        }
    }

    #[test]
    fn all_ginza_shapes_triangulate() {
        let r = super::super::replay::parse(include_str!(
            "../../tests/fixtures/ginzabattle02.json"
        ))
        .unwrap();
        for s in &r.map.shapes {
            let tris = triangulate(&s.ring);
            assert_eq!(tris.len(), s.ring.len() - 2, "shape {}", s.id);
            let area: f64 = tris.iter().map(|&t| tri_area(&s.ring, t)).sum();
            let want = shoelace(&s.ring).abs();
            assert!(
                (area - want).abs() <= 1e-6 * want,
                "shape {}: {area} vs {want}",
                s.id
            );
        }
    }

    #[test]
    fn clip_segment_cases() {
        assert_eq!(
            clip_segment([-10.0, 5.0], [20.0, 5.0], 10.0, 10.0),
            Some(([0.0, 5.0], [10.0, 5.0]))
        );
        assert_eq!(
            clip_segment([1.0, 1.0], [2.0, 2.0], 10.0, 10.0),
            Some(([1.0, 1.0], [2.0, 2.0]))
        );
        assert_eq!(clip_segment([-5.0, -5.0], [-1.0, 20.0], 10.0, 10.0), None);
    }

    #[test]
    fn strip_is_clipped_to_arena() {
        let s = Space::new(Some(1.0));
        let m = strip(
            &[[-100.0, 25.0], [100.0, 25.0]],
            10.0,
            (50.0, 50.0),
            &s,
            0.03,
        )
        .unwrap();
        check_closed(&m);
        assert!(
            m.positions
                .iter()
                .all(|p| (0.0..=50.0).contains(&p[0]) && p[1] == 0.03)
        );
        assert!(
            m.positions.iter().any(|p| p[2] == 20.0)
                && m.positions.iter().any(|p| p[2] == 30.0)
        );
        for t in 0..m.indices.len() / 3 {
            assert!(triangle_normal(&m, t)[1] > 0.0);
        }
        assert_eq!(
            strip(
                &[[-100.0, -25.0], [100.0, -25.0]],
                10.0,
                (50.0, 50.0),
                &s,
                0.03
            ),
            None
        );
    }

    #[test]
    fn ginza_road_count() {
        let r = super::super::replay::parse(include_str!(
            "../../tests/fixtures/ginzabattle02.json"
        ))
        .unwrap();
        let s = Space::new(r.map.px_per_m);
        let n = r
            .map
            .road_lines
            .iter()
            .filter(|l| {
                strip(&l.points, l.width, (r.width, r.height), &s, 0.03)
                    .is_some()
            })
            .count();
        assert_eq!(n, 166);
    }
}
