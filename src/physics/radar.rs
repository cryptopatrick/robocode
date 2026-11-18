//! The radar beam: the region a robot's radar swept this turn and could see,
//! cut off by buildings. Pure geometry, no engine state. The recording stores
//! the polygon and the viewer draws it, so it must never under-draw: every
//! robot the scan finds has its centre inside it.

use std::f64::consts::PI;

use crate::geometry::{Battlefield, Point, Rect};
use crate::rules::{RADAR_SCAN_RADIUS, ROBOT_RADIUS};

/// How far either side of a corner the extra rays go, in radians.
const RAY_EPSILON: f64 = 1e-5;
/// The widest step between rays along the outer edge, in degrees.
const MAX_FAN_STEP: f64 = 2.0;

/// The polygon of what the radar swept from `origin` and could see: the arc
/// from `from_heading` turning by `sweep` degrees (signed; headings are 0° up,
/// clockwise), widened by a robot's radius in angle and range, clipped to the
/// field and cut off at the first building on each ray. Vertices go round
/// `origin` in angle order.
pub fn radar_beam(
    origin: Point,
    from_heading: f64,
    sweep: f64,
    buildings: &[Rect],
    field: &Battlefield,
) -> Vec<Point> {
    let region = target_region(origin, from_heading, sweep, field);
    if region.len() < 3 {
        return Vec::new();
    }
    let edges = Edges::of(&region);
    // Only buildings that reach into the region's bounding box can stop a ray
    // before it leaves the region.
    let (lo, hi) = bounding_box(&region);
    let buildings: Vec<Rect> = buildings
        .iter()
        .filter(|b| {
            b.max.x >= lo.x
                && b.min.x <= hi.x
                && b.max.y >= lo.y
                && b.min.y <= hi.y
        })
        .copied()
        .collect();

    // Ray angles: the region's vertices, building corners inside it, and the
    // points where edges cross (where the nearest boundary switches).
    let mut events: Vec<Point> = region.clone();
    for b in &buildings {
        events.extend(
            corners(b).into_iter().filter(|&c| edges.strictly_inside(c)),
        );
    }
    events.extend(
        crossings(&region, &buildings)
            .into_iter()
            .filter(|&p| edges.inside_or_on(p)),
    );

    let mut angles: Vec<f64> = Vec::with_capacity(events.len() * 3);
    for p in events {
        if p == origin {
            continue;
        }
        let theta = heading_rad(origin, p);
        for a in [theta - RAY_EPSILON, theta, theta + RAY_EPSILON] {
            angles.push(wrap(a));
        }
    }
    angles.sort_by(f64::total_cmp);
    angles.dedup();

    let reach = (RADAR_SCAN_RADIUS + 2.0 * ROBOT_RADIUS) * 2.0;
    let hits = angles
        .into_iter()
        .map(|a| {
            let dir = Point::new(a.sin(), -a.cos());
            let far =
                Point::new(origin.x + dir.x * reach, origin.y + dir.y * reach);
            let mut length = edges.exit_distance(origin, dir);
            for b in &buildings {
                if let Some(t) = b.first_contact(origin, far) {
                    length = length.min(t * reach);
                }
            }
            Point::new(origin.x + dir.x * length, origin.y + dir.y * length)
        })
        .collect();
    merge_collinear(hits)
}

/// The convex region every point within a robot's radius of the swept sector
/// lies in: the hull of the square `origin ± R` and a fan at radius
/// `(S + R) / cos(Δ/2)` (chords tangent to the circle of radius `S + R`) from
/// `lo − β` to `hi + β`, clipped to the field.
fn target_region(
    origin: Point,
    from_heading: f64,
    sweep: f64,
    field: &Battlefield,
) -> Vec<Point> {
    let (r, s) = (ROBOT_RADIUS, RADAR_SCAN_RADIUS);
    let (lo, hi) = if sweep >= 0.0 {
        (from_heading, from_heading + sweep)
    } else {
        (from_heading + sweep, from_heading)
    };
    let beta = (r / (s - r)).asin().to_degrees();
    let span = hi - lo + 2.0 * beta;
    let steps = (span / MAX_FAN_STEP).ceil().max(1.0) as usize;
    let step = span / steps as f64;
    let radius = (s + r) / (step.to_radians() / 2.0).cos();

    let mut points = vec![
        Point::new(origin.x - r, origin.y - r),
        Point::new(origin.x + r, origin.y - r),
        Point::new(origin.x + r, origin.y + r),
        Point::new(origin.x - r, origin.y + r),
    ];
    points.extend(
        (0..=steps)
            .map(|i| origin.translate(lo - beta + step * i as f64, radius)),
    );
    let hull = convex_hull(points);
    clip_to_field(hull, field)
}

/// Andrew's monotone chain; counter-clockwise in x-right, y-up terms.
fn convex_hull(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let mut lower: Vec<Point> = Vec::new();
    for &p in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0
        {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<Point> = Vec::new();
    for &p in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0
        {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Sutherland–Hodgman against the four walls.
fn clip_to_field(mut poly: Vec<Point>, field: &Battlefield) -> Vec<Point> {
    // Each wall as how far a point is on its inner side.
    let walls: [&dyn Fn(Point) -> f64; 4] = [
        &|p: Point| p.x,
        &|p: Point| field.width - p.x,
        &|p: Point| p.y,
        &|p: Point| field.height - p.y,
    ];
    for inside in walls {
        if poly.is_empty() {
            break;
        }
        let mut out = Vec::with_capacity(poly.len() + 2);
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let (da, db) = (inside(a), inside(b));
            if da >= 0.0 {
                out.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                let t = da / (da - db);
                out.push(Point::new(
                    a.x + (b.x - a.x) * t,
                    a.y + (b.y - a.y) * t,
                ));
            }
        }
        // A crossing can land a rounding error from a vertex; a leftover
        // sliver edge could point backwards and break the inside tests.
        out.dedup_by(|b, a| near(*a, *b));
        while out.len() > 1 && near(out[0], out[out.len() - 1]) {
            out.pop();
        }
        poly = out;
    }
    poly
}

/// A convex polygon's edges with outward normals.
struct Edges {
    /// (start point, outward normal, normal length).
    edges: Vec<(Point, Point, f64)>,
}

impl Edges {
    fn of(poly: &[Point]) -> Self {
        let n = poly.len();
        let area: f64 = (0..n)
            .map(|i| {
                let (a, b) = (poly[i], poly[(i + 1) % n]);
                a.x * b.y - b.x * a.y
            })
            .sum();
        let sign = if area > 0.0 { 1.0 } else { -1.0 };
        let edges = (0..n)
            .map(|i| {
                let (a, b) = (poly[i], poly[(i + 1) % n]);
                let normal =
                    Point::new(sign * (b.y - a.y), -sign * (b.x - a.x));
                (a, normal, normal.x.hypot(normal.y))
            })
            .collect();
        Self { edges }
    }

    /// Signed distances of `p` from each edge line; negative is inside.
    fn offsets(&self, p: Point) -> impl Iterator<Item = f64> + '_ {
        self.edges.iter().map(move |&(a, n, len)| {
            (n.x * (p.x - a.x) + n.y * (p.y - a.y)) / len
        })
    }

    fn strictly_inside(&self, p: Point) -> bool {
        self.offsets(p).all(|d| d < 0.0)
    }

    fn inside_or_on(&self, p: Point) -> bool {
        self.offsets(p).all(|d| d <= 1e-6)
    }

    /// How far a ray from `origin` (inside the polygon) along unit `dir` goes
    /// before it leaves.
    fn exit_distance(&self, origin: Point, dir: Point) -> f64 {
        self.edges
            .iter()
            .filter_map(|&(a, n, _)| {
                let toward = n.x * dir.x + n.y * dir.y;
                (toward > 0.0).then(|| {
                    (n.x * (a.x - origin.x) + n.y * (a.y - origin.y)) / toward
                })
            })
            .fold(f64::INFINITY, f64::min)
            .max(0.0)
    }
}

fn corners(b: &Rect) -> [Point; 4] {
    [b.min, Point::new(b.max.x, b.min.y), b.max, Point::new(b.min.x, b.max.y)]
}

/// Every point where a building edge touches another building's edge or the
/// region's boundary. Between two neighbouring ray angles the nearest
/// boundary must stay on one straight edge, and it can only switch to another
/// at a vertex, a corner or one of these crossings.
fn crossings(region: &[Point], buildings: &[Rect]) -> Vec<Point> {
    let region_edges: Vec<(Point, Point)> = (0..region.len())
        .map(|i| (region[i], region[(i + 1) % region.len()]))
        .collect();
    let building_edges: Vec<(Point, Point)> = buildings
        .iter()
        .flat_map(|b| {
            let c = corners(b);
            (0..4).map(move |i| (c[i], c[(i + 1) % 4]))
        })
        .collect();
    let mut points = Vec::new();
    for (i, &(p1, p2)) in building_edges.iter().enumerate() {
        let others = building_edges[i + 1..].iter().chain(region_edges.iter());
        for &(q1, q2) in others {
            if let Some(p) = intersection(p1, p2, q1, q2) {
                points.push(p);
            }
        }
    }
    points
}

/// Where segments p1–p2 and q1–q2 meet, if they do and aren't parallel.
fn intersection(p1: Point, p2: Point, q1: Point, q2: Point) -> Option<Point> {
    let r = Point::new(p2.x - p1.x, p2.y - p1.y);
    let s = Point::new(q2.x - q1.x, q2.y - q1.y);
    let denom = r.x * s.y - r.y * s.x;
    if denom.abs() < 1e-12 {
        return None;
    }
    let w = Point::new(q1.x - p1.x, q1.y - p1.y);
    let t = (w.x * s.y - w.y * s.x) / denom;
    let u = (w.x * r.y - w.y * r.x) / denom;
    let tol = 1e-9;
    ((-tol..=1.0 + tol).contains(&t) && (-tol..=1.0 + tol).contains(&u))
        .then(|| Point::new(p1.x + r.x * t, p1.y + r.y * t))
}

fn bounding_box(poly: &[Point]) -> (Point, Point) {
    poly.iter().fold(
        (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                Point::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    )
}

/// The heading from `from` to `to` in radians (0 up, clockwise).
fn heading_rad(from: Point, to: Point) -> f64 {
    (to.x - from.x).atan2(-(to.y - from.y))
}

/// An angle in radians, in [-π, π).
fn wrap(a: f64) -> f64 {
    a - 2.0 * PI * ((a + PI) / (2.0 * PI)).floor()
}

/// Points closer than this are the same vertex.
fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-7 && (a.y - b.y).abs() < 1e-7
}

fn cross(o: Point, a: Point, b: Point) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// Whether `b` lies on the segment a–c (up to rounding), so it can go.
fn redundant(a: Point, b: Point, c: Point) -> bool {
    let (dx, dy) = (c.x - a.x, c.y - a.y);
    let between = (b.x - a.x) * (c.x - b.x) + (b.y - a.y) * (c.y - b.y) >= 0.0;
    between && cross(a, b, c).abs() < 1e-9 * (dx * dx + dy * dy)
}

/// Drops the middle points of straight runs, keeping each run's two ends.
fn merge_collinear(points: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        while out.len() >= 2
            && redundant(out[out.len() - 2], out[out.len() - 1], p)
        {
            out.pop();
        }
        out.push(p);
    }
    // The polygon is closed: tidy the join between the last and first points.
    while out.len() > 3 {
        let n = out.len();
        if redundant(out[n - 2], out[n - 1], out[0]) {
            out.pop();
        } else if redundant(out[n - 1], out[0], out[1]) {
            out.remove(0);
        } else {
            break;
        }
    }
    out
}
