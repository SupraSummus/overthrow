//! Hex ↔ screen-pixel mapping for the frontend.
//!
//! The engine works in cube coordinates and has no notion of pixels;
//! screen placement and click hit-testing are purely a UI concern,
//! so they live here rather than in `engine/`.
//! Pointy-top layout, using the standard axial-to-pixel formulas
//! (see redblobgames.com/grids/hexagons);
//! `q = hex.x`, `r = hex.z()` form the axial pair.
//! `size` is the hex circumradius (center to corner),
//! which is also what macroquad's `draw_poly` takes as radius.

use overthrow_engine::Hex;

const SQRT_3: f32 = 1.732_050_8;

#[derive(Clone, Copy)]
pub struct Layout {
    /// Circumradius of one hex, in pixels.
    pub size: f32,
    /// Screen position of the map center (hex 0,0).
    pub origin: (f32, f32),
}

impl Layout {
    /// Center of `hex` in screen pixels.
    pub fn center(&self, hex: Hex) -> (f32, f32) {
        let (q, r) = (hex.x as f32, hex.z() as f32);
        let x = self.size * SQRT_3 * (q + r / 2.0);
        let y = self.size * 1.5 * r;
        (self.origin.0 + x, self.origin.1 + y)
    }

    /// The hex whose cell contains screen point `(px, py)`.
    /// Always returns a hex (the nearest one);
    /// callers gate on `Hex::in_radius` for the map.
    pub fn hex_at(&self, px: f32, py: f32) -> Hex {
        let (dx, dy) = (px - self.origin.0, py - self.origin.1);
        let q = (SQRT_3 / 3.0 * dx - dy / 3.0) / self.size;
        let r = (2.0 / 3.0 * dy) / self.size;
        cube_round(q, r)
    }
}

/// Round fractional axial (q, r) to the nearest hex via cube rounding:
/// round all three cube components,
/// then correct the one with the largest rounding error
/// so the constraint x + y + z == 0 holds.
fn cube_round(q: f32, r: f32) -> Hex {
    let (x, z, y) = (q, r, -q - r);
    let (mut rx, mut ry, mut rz) = (x.round(), y.round(), z.round());
    let (dx, dy, dz) = ((rx - x).abs(), (ry - y).abs(), (rz - z).abs());
    if dx > dy && dx > dz {
        rx = -ry - rz;
    } else if dy > dz {
        ry = -rx - rz;
    } else {
        rz = -rx - ry;
    }
    let _ = rz; // z is derived by the engine (Hex stores x and y).
    Hex::new(rx as i32, ry as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use overthrow_engine::coords::hexagon;

    fn layout() -> Layout {
        Layout {
            size: 30.0,
            origin: (400.0, 300.0),
        }
    }

    #[test]
    fn center_of_hex_maps_back_to_itself() {
        // A click at a hex's own center must resolve to that hex,
        // for every tile of a real map.
        let l = layout();
        for hex in hexagon(5) {
            let (cx, cy) = l.center(hex);
            assert_eq!(l.hex_at(cx, cy), hex, "round-trip failed for {hex:?}");
        }
    }

    #[test]
    fn small_jitter_stays_on_the_same_hex() {
        // Points near a center (well inside the cell) resolve to that hex.
        let l = layout();
        for hex in hexagon(3) {
            let (cx, cy) = l.center(hex);
            for (jx, jy) in [(5.0, 0.0), (-5.0, 4.0), (0.0, -6.0)] {
                assert_eq!(l.hex_at(cx + jx, cy + jy), hex);
            }
        }
    }
}
