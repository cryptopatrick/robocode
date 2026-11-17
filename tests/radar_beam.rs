//! The radar beam polygon: the swept region the radar can see, cut off by
//! buildings and widened by a robot's radius so it never under-draws.

use robocode::physics::radar::radar_beam;
use robocode::rules::ROBOT_RADIUS;
use robocode::{Battlefield, Point, Rect};

/// Even-odd point-in-polygon test.
fn contains(poly: &[Point], p: Point) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

fn cross(o: Point, a: Point, b: Point) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// Whether segments p1–p2 and q1–q2 touch or cross.
fn segments_touch(p1: Point, p2: Point, q1: Point, q2: Point) -> bool {
    let on = |a: Point, b: Point, p: Point| {
        p.x >= a.x.min(b.x)
            && p.x <= a.x.max(b.x)
            && p.y >= a.y.min(b.y)
            && p.y <= a.y.max(b.y)
    };
    let (d1, d2) = (cross(q1, q2, p1), cross(q1, q2, p2));
    let (d3, d4) = (cross(p1, p2, q1), cross(p1, p2, q2));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    (d1 == 0.0 && on(q1, q2, p1))
        || (d2 == 0.0 && on(q1, q2, p2))
        || (d3 == 0.0 && on(p1, p2, q1))
        || (d4 == 0.0 && on(p1, p2, q2))
}

/// No NaN, at least a triangle, and no two non-adjacent edges touch.
fn assert_simple(poly: &[Point]) {
    assert!(poly.len() >= 3, "only {} vertices", poly.len());
    assert!(
        poly.iter().all(|p| p.x.is_finite() && p.y.is_finite()),
        "{poly:?}"
    );
    let n = poly.len();
    for i in 0..n {
        for j in i + 1..n {
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            if adjacent {
                continue;
            }
            let (p1, p2) = (poly[i], poly[(i + 1) % n]);
            let (q1, q2) = (poly[j], poly[(j + 1) % n]);
            assert!(
                !segments_touch(p1, p2, q1, q2),
                "edges {i} and {j} intersect: {p1:?}-{p2:?} / {q1:?}-{q2:?}"
            );
        }
    }
}

#[test]
fn open_field_beam_covers_the_dilated_wedge() {
    // The brief's heading 0 points up, off the field from y = 300; the same
    // check looks down instead, on a field big enough for the full range:
    // from 180° turning left 45° to 135°.
    let field = Battlefield::new(2000.0, 2000.0);
    let origin = Point::new(400.0, 300.0);
    let beam = radar_beam(origin, 180.0, -45.0, &[], &field);
    assert_simple(&beam);
    assert!(contains(&beam, Point::new(400.0, 300.0 + 1200.0 * 0.99)));
    // 17 px outside each edge ray (headings 135 and 180), 600 px out.
    for (heading, outward) in [(135.0, -90.0), (180.0, 90.0)] {
        let p = origin
            .translate(heading, 600.0)
            .translate(heading + outward, 17.0);
        assert!(contains(&beam, p), "{p:?} outside the edge at {heading}");
    }
    assert!(!contains(&beam, Point::new(400.0, 300.0 - 100.0)));
}

#[test]
fn building_casts_a_shadow() {
    let field = Battlefield::new(800.0, 600.0);
    let building = Rect::new(300.0, 280.0, 40.0, 40.0);
    let beam =
        radar_beam(Point::new(100.0, 300.0), 90.0, 10.0, &[building], &field);
    assert_simple(&beam);
    assert!(!contains(&beam, Point::new(500.0, 300.0)), "shadow is drawn");
    assert!(contains(&beam, Point::new(250.0, 300.0)), "in front of it");
    // Beside the building, inside the sweep (bearing 98.5°).
    assert!(contains(&beam, Point::new(500.0, 360.0)), "beside it");
}

#[test]
fn beam_next_to_a_corner_is_well_formed() {
    let field = Battlefield::new(800.0, 600.0);
    let building = Rect::new(300.0, 300.0, 50.0, 50.0);
    // Sweep from 90° to 135°: the corner sits on the trailing edge ray.
    let beam =
        radar_beam(Point::new(279.0, 279.0), 90.0, 45.0, &[building], &field);
    assert_simple(&beam);
}

#[test]
fn stationary_radar_has_a_thin_beam() {
    let field = Battlefield::new(1200.0, 600.0);
    let origin = Point::new(100.0, 300.0);
    let beam = radar_beam(origin, 90.0, 0.0, &[], &field);
    assert!(!beam.is_empty());
    assert_simple(&beam);
    assert!(contains(&beam, origin.translate(90.0, 1000.0)));
}

/// A building whose edge crosses the side of the beam, with no corner of the
/// beam or the building at the crossing. The visible area in front of the
/// building, next to that crossing, must not be cut off.
#[test]
fn building_across_the_beam_edge_leaves_no_gap() {
    // Tall enough that the field's top wall doesn't add a ray of its own.
    let field = Battlefield::new(800.0, 1400.0);
    let origin = Point::new(400.0, 1350.0);
    // Left of the heading-0 ray, from x = 350 to 395, y = 950 to 1050.
    let building = Rect::new(350.0, 950.0, 45.0, 100.0);
    let beam = radar_beam(origin, 0.0, 45.0, &[building], &field);
    assert_simple(&beam);
    // 10 px left of the leading edge ray, 50 px in front of the building:
    // within a robot's radius of the swept sector and in clear view.
    let target = Point::new(390.0, 1100.0);
    assert!(robocode::line_of_sight(origin, target, &[building]));
    assert!(contains(&beam, target), "{target:?} cut off: {beam:?}");
}

/// A robot against the bottom wall: clipping the beam to the field must not
/// leave a sliver edge that collapses the beam to nothing.
#[test]
fn beam_from_a_robot_on_the_wall_reaches_out() {
    let field = Battlefield::new(1200.0, 600.0);
    let origin = Point::new(233.89660123919523, 600.0 - ROBOT_RADIUS);
    let beam =
        radar_beam(origin, 73.50583036278977, 24.19564004270258, &[], &field);
    assert_simple(&beam);
    assert!(contains(&beam, origin.translate(85.0, 900.0)), "{beam:?}");
}
