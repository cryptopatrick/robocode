//! Map objects: buildings that block robots and bullets, and roads that only
//! show on the map. The default battle has no terrain (an empty arena inside
//! the four walls). The urban map is one fixed layout, so battles stay
//! reproducible.

use crate::geometry::{Ratio, Rect};

/// Buildings and roads, in battlefield pixels.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Terrain {
    pub buildings: Vec<Rect>,
    pub roads: Vec<Rect>,
}

/// Buildings as `(x, y, width, height)`, one table per ratio, by block.
const URBAN_4X3: [(f64, f64, f64, f64); 9] = [
    // north-west block
    (200.0, 60.0, 120.0, 80.0),
    (200.0, 170.0, 60.0, 60.0),
    (300.0, 170.0, 50.0, 50.0),
    // north-east block
    (445.0, 60.0, 150.0, 70.0),
    (445.0, 160.0, 70.0, 60.0),
    // south-west block
    (60.0, 345.0, 90.0, 120.0),
    (190.0, 390.0, 110.0, 50.0),
    // south-east block
    (475.0, 345.0, 120.0, 80.0),
    (475.0, 460.0, 60.0, 50.0),
];

const URBAN_2X1: [(f64, f64, f64, f64); 15] = [
    // north-west block
    (200.0, 60.0, 120.0, 80.0),
    (200.0, 170.0, 60.0, 60.0),
    (300.0, 170.0, 50.0, 50.0),
    (400.0, 60.0, 110.0, 50.0),
    // north-east block
    (690.0, 60.0, 150.0, 70.0),
    (690.0, 160.0, 70.0, 60.0),
    (900.0, 60.0, 120.0, 80.0),
    // south-west block
    (60.0, 345.0, 90.0, 120.0),
    (190.0, 390.0, 110.0, 50.0),
    (360.0, 350.0, 60.0, 50.0),
    (400.0, 460.0, 120.0, 80.0),
    // south-east block
    (675.0, 345.0, 120.0, 80.0),
    (675.0, 460.0, 60.0, 50.0),
    (850.0, 345.0, 70.0, 60.0),
    (900.0, 460.0, 110.0, 50.0),
];

const URBAN_16X9: [(f64, f64, f64, f64); 14] = [
    // north-west block
    (200.0, 60.0, 120.0, 80.0),
    (200.0, 170.0, 60.0, 60.0),
    (300.0, 170.0, 50.0, 50.0),
    (380.0, 60.0, 60.0, 60.0),
    // north-east block
    (620.0, 60.0, 150.0, 70.0),
    (620.0, 160.0, 70.0, 60.0),
    (800.0, 160.0, 60.0, 60.0),
    // south-west block
    (60.0, 345.0, 90.0, 120.0),
    (190.0, 390.0, 110.0, 50.0),
    (340.0, 350.0, 60.0, 50.0),
    // south-east block
    (620.0, 345.0, 120.0, 80.0),
    (620.0, 460.0, 60.0, 50.0),
    (790.0, 380.0, 70.0, 60.0),
    (790.0, 470.0, 60.0, 50.0),
];

const URBAN_1X1: [(f64, f64, f64, f64); 10] = [
    // north-west block
    (150.0, 60.0, 70.0, 60.0),
    (150.0, 160.0, 60.0, 60.0),
    (60.0, 170.0, 50.0, 50.0),
    // north-east block
    (380.0, 60.0, 70.0, 60.0),
    (380.0, 160.0, 50.0, 50.0),
    (470.0, 170.0, 60.0, 60.0),
    // south-west block
    (60.0, 345.0, 90.0, 120.0),
    (190.0, 400.0, 60.0, 50.0),
    // south-east block
    (380.0, 345.0, 120.0, 80.0),
    (380.0, 460.0, 60.0, 50.0),
];

fn rects(table: &[(f64, f64, f64, f64)]) -> Vec<Rect> {
    table.iter().map(|&(x, y, w, h)| Rect::new(x, y, w, h)).collect()
}

fn roads(width: f64, height: f64) -> Vec<Rect> {
    let (cx, cy) = (width / 2.0, height / 2.0);
    vec![
        Rect::new(0.0, cy - 25.0, width, 50.0),
        Rect::new(cx - 25.0, 0.0, 50.0, height),
    ]
}

impl Terrain {
    /// The hand-placed urban map for a battlefield ratio.
    pub fn urban_for(ratio: Ratio) -> Self {
        let (w, h) = ratio.size();
        let table: &[(f64, f64, f64, f64)] = match ratio {
            Ratio::R4x3 => &URBAN_4X3,
            Ratio::R2x1 => &URBAN_2X1,
            Ratio::R16x9 => &URBAN_16X9,
            Ratio::R1x1 => &URBAN_1X1,
        };
        Self { buildings: rects(table), roads: roads(w, h) }
    }

    /// The urban map for a `width` × `height` arena. A ratio size gets that
    /// ratio's table; any other size keeps the 0.5.0 layout anchored to the
    /// centre, filtered to the blocks that fit and leave the spawn corners clear.
    pub fn urban(width: f64, height: f64) -> Self {
        if let Some(ratio) = Ratio::from_size(width, height) {
            return Self::urban_for(ratio);
        }
        let (cx, cy) = (width / 2.0, height / 2.0);
        let roads = roads(width, height);
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
                let fits = b.min.x >= 0.0
                    && b.min.y >= 0.0
                    && b.max.x <= width
                    && b.max.y <= height;
                let clear = spawns.iter().all(|s| {
                    let g = b.grow(crate::rules::ROBOT_RADIUS);
                    !(s.position.x >= g.min.x
                        && s.position.x <= g.max.x
                        && s.position.y >= g.min.y
                        && s.position.y <= g.max.y)
                });
                fits && clear
            })
            .collect();
        Self { buildings, roads }
    }
}
