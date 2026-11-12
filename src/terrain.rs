//! Map objects: buildings that block robots and bullets, and roads that only
//! show on the map. The default battle has no terrain (an empty arena inside
//! the four walls). The urban map is one fixed layout, so battles stay
//! reproducible.

use crate::geometry::Rect;

/// Buildings and roads, in battlefield pixels.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Terrain {
    pub buildings: Vec<Rect>,
    pub roads: Vec<Rect>,
}

impl Terrain {
    /// The fixed urban map for a `width` × `height` arena. Roads cross the
    /// middle; buildings fill the blocks between them, and the corners
    /// around the spawn points are kept clear.
    pub fn urban(width: f64, height: f64) -> Self {
        let (cx, cy) = (width / 2.0, height / 2.0);
        let roads = vec![
            Rect::new(0.0, cy - 25.0, width, 50.0),
            Rect::new(cx - 25.0, 0.0, 50.0, height),
        ];
        let buildings = vec![
            // north-west block
            Rect::new(200.0, 60.0, 120.0, 80.0),
            Rect::new(200.0, 170.0, 60.0, 60.0),
            Rect::new(300.0, 170.0, 50.0, 50.0),
            // north-east block
            Rect::new(cx + 45.0, 60.0, 150.0, 70.0),
            Rect::new(cx + 45.0, 160.0, 70.0, 60.0),
            // south-west block
            Rect::new(60.0, cy + 45.0, 90.0, 120.0),
            Rect::new(190.0, cy + 90.0, 110.0, 50.0),
            // south-east block
            Rect::new(cx + 75.0, cy + 45.0, 120.0, 80.0),
            Rect::new(cx + 75.0, cy + 160.0, 60.0, 50.0),
        ];
        // Smaller arenas keep only the blocks that fit inside them and leave the
        // spawn corners free, so every robot can start and move.
        let spawns = crate::roster::spawns(4, width, height);
        let buildings = buildings
            .into_iter()
            .filter(|b| {
                let fits = b.min.x >= 0.0 && b.min.y >= 0.0 && b.max.x <= width && b.max.y <= height;
                let clear = spawns.iter().all(|s| {
                    let g = b.grow(crate::rules::ROBOT_RADIUS);
                    !(s.position.x >= g.min.x && s.position.x <= g.max.x && s.position.y >= g.min.y && s.position.y <= g.max.y)
                });
                fits && clear
            })
            .collect();
        Self { buildings, roads }
    }
}
