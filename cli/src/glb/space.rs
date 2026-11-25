//! Units, axes and rotations: recording pixels and degrees to glTF metres and quaternions.
//!
//! Recordings use screen axes (`x` east, `y` south, north is `−y`). Arena `x` maps to
//! glTF `+X`, arena `y` to glTF `+Z`, and Y is up. Every robot part looks along local `−Z`.

/// Pixels per metre when the recording's map does not say.
pub const DEFAULT_PX_PER_M: f64 = 4.0;
/// Keyframe rate: one recorded turn every 1/30 s.
pub const TURNS_PER_SECOND: f64 = 30.0;

/// The recording's scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Space {
    pub m_per_px: f64,
}

impl Space {
    pub fn new(px_per_m: Option<f64>) -> Space {
        Space { m_per_px: 1.0 / px_per_m.unwrap_or(DEFAULT_PX_PER_M) }
    }

    /// A recording point (px) at a height in metres, as a glTF position.
    pub fn point(&self, x_px: f64, y_px: f64, up_m: f64) -> [f32; 3] {
        [
            (x_px * self.m_per_px) as f32,
            up_m as f32,
            (y_px * self.m_per_px) as f32,
        ]
    }

    /// A length in px, in metres.
    pub fn len(&self, px: f64) -> f64 {
        px * self.m_per_px
    }
}

/// The rotation for a heading (degrees, 0 = north, clockwise): `−heading` about `+Y`, as `[x, y, z, w]`.
pub fn yaw(heading_deg: f64) -> [f32; 4] {
    let half = -heading_deg.to_radians() / 2.0;
    [0.0, half.sin() as f32, 0.0, half.cos() as f32]
}

/// The rotation whose `−Z` points along the screen direction `(dx, dy)`.
pub fn yaw_toward(dx_px: f64, dy_px: f64) -> [f32; 4] {
    yaw(dx_px.atan2(-dy_px).to_degrees())
}

/// `next`, negated when that keeps it on the same side as `prev`, so interpolation takes the short way.
pub fn continuous(prev: [f32; 4], next: [f32; 4]) -> [f32; 4] {
    let dot: f32 = prev.iter().zip(next).map(|(a, b)| a * b).sum();
    if dot < 0.0 { next.map(|c| -c) } else { next }
}

/// The time of turn `k` in seconds.
pub fn turn_time(k: usize) -> f32 {
    (k as f64 / TURNS_PER_SECOND) as f32
}

/// `"#rrggbb"` (sRGB) as linear RGBA with alpha 1.
pub fn srgb_hex(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    let channel = |i: usize| {
        let c = f64::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0))
            / 255.0;
        let linear = if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        };
        linear as f32
    };
    [channel(0), channel(2), channel(4), 1.0]
}

#[cfg(test)]
pub(crate) fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let [x, y, z, w] = q;
    let u = [x, y, z];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let t = cross(u, v).map(|c| 2.0 * c);
    let c = cross(u, t);
    [v[0] + w * t[0] + c[0], v[1] + w * t[1] + c[1], v[2] + w * t[2] + c[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[f32], b: &[f32], eps: f32) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < eps)
    }

    fn dot(a: [f32; 4], b: [f32; 4]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    const FORWARD: [f32; 3] = [0.0, 0.0, -1.0];

    #[test]
    fn point_maps_axes() {
        assert_eq!(
            Space::new(Some(4.0)).point(100.0, 200.0, 0.5),
            [25.0, 0.5, 50.0]
        );
        assert_eq!(Space::new(None).m_per_px, 0.25);
        assert_eq!(Space::new(Some(10.0)).len(30.0), 3.0);
    }

    #[test]
    fn heading_matches_recorded_motion() {
        // GinzaBattle02: robot 1 at heading 42.5° moved (+3.38, −3.69) px in one turn.
        let s = Space::new(None);
        let m = s.point(3.38, -3.69, 0.0);
        let n = (m[0] * m[0] + m[2] * m[2]).sqrt();
        let dir = [m[0] / n, 0.0, m[2] / n];
        assert!(close(&rotate(yaw(42.5), FORWARD), &dir, 1e-2));
        assert!(close(&rotate(yaw(90.0), FORWARD), &[1.0, 0.0, 0.0], 1e-5));
        assert!(close(&rotate(yaw(180.0), FORWARD), &[0.0, 0.0, 1.0], 1e-5));
        assert!(close(&yaw(0.0), &[0.0, 0.0, 0.0, 1.0], 1e-6));
    }

    #[test]
    fn yaw_toward_east() {
        assert!(close(
            &rotate(yaw_toward(1.0, 0.0), FORWARD),
            &[1.0, 0.0, 0.0],
            1e-5
        ));
        assert!(close(
            &rotate(yaw_toward(0.0, 2.0), FORWARD),
            &[0.0, 0.0, 1.0],
            1e-5
        ));
    }

    #[test]
    fn continuous_takes_short_way() {
        let a = yaw(359.0);
        assert!(dot(a, continuous(a, yaw(1.0))) > 0.99);
        assert_eq!(continuous(a, yaw(358.0)), yaw(358.0));
    }

    #[test]
    fn turn_time_is_thirtieths() {
        assert_eq!(turn_time(30), 1.0);
        assert_eq!(turn_time(0), 0.0);
    }

    #[test]
    fn srgb_hex_converts() {
        assert_eq!(srgb_hex("#ffffff"), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(srgb_hex("#000000"), [0.0, 0.0, 0.0, 1.0]);
        assert!((srgb_hex("#808080")[0] - 0.2159).abs() < 1e-3);
    }
}
