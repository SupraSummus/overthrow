//! Hex grid in cube coordinates (x + y + z == 0). Only x and y are stored;
//! z is derived. This matches the coordinate system of the original Python
//! implementation in `old/overthrow/games/coords.py`.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Hex {
    pub x: i32,
    pub y: i32,
}

impl Hex {
    pub const fn new(x: i32, y: i32) -> Self {
        Hex { x, y }
    }

    pub const fn z(self) -> i32 {
        -self.x - self.y
    }

    pub fn distance(self, other: Hex) -> i32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        let dz = (self.z() - other.z()).abs();
        dx.max(dy).max(dz)
    }

    pub fn neighbor(self, dir: Direction) -> Hex {
        let (dx, dy) = dir.delta();
        Hex::new(self.x + dx, self.y + dy)
    }

    /// The six adjacent hexes with their directions (map bounds not checked).
    pub fn neighbors(self) -> impl Iterator<Item = (Direction, Hex)> {
        Direction::ALL
            .into_iter()
            .map(move |d| (d, self.neighbor(d)))
    }

    /// Whether the hex lies within a hexagonal map of the given radius.
    pub fn in_radius(self, radius: i32) -> bool {
        self.x.abs() <= radius && self.y.abs() <= radius && self.z().abs() <= radius
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Direction {
    East,
    NorthEast,
    NorthWest,
    West,
    SouthWest,
    SouthEast,
}

impl Direction {
    /// All directions, in ring order (each is adjacent to the next).
    pub const ALL: [Direction; 6] = [
        Direction::East,
        Direction::NorthEast,
        Direction::NorthWest,
        Direction::West,
        Direction::SouthWest,
        Direction::SouthEast,
    ];

    pub const fn delta(self) -> (i32, i32) {
        match self {
            Direction::East => (1, -1),
            Direction::NorthEast => (1, 0),
            Direction::NorthWest => (0, 1),
            Direction::West => (-1, 1),
            Direction::SouthWest => (-1, 0),
            Direction::SouthEast => (0, -1),
        }
    }
}

/// All hexes of a hexagonal map with the given radius, in a deterministic order.
pub fn hexagon(radius: i32) -> impl Iterator<Item = Hex> {
    (-radius..=radius).flat_map(move |x| {
        (-radius..=radius)
            .map(move |y| Hex::new(x, y))
            .filter(move |h| h.in_radius(radius))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexagon_tile_count() {
        // 3r^2 + 3r + 1
        for r in 0..6 {
            let expected = (3 * r * r + 3 * r + 1) as usize;
            assert_eq!(hexagon(r).count(), expected);
        }
    }

    #[test]
    fn neighbors_are_distance_one() {
        let h = Hex::new(2, -1);
        for dir in Direction::ALL {
            assert_eq!(h.distance(h.neighbor(dir)), 1);
        }
    }

    #[test]
    fn directions_are_distinct_and_sum_to_zero() {
        let mut seen = std::collections::HashSet::new();
        let (mut sx, mut sy) = (0, 0);
        for dir in Direction::ALL {
            let d = dir.delta();
            assert!(seen.insert(d));
            sx += d.0;
            sy += d.1;
        }
        assert_eq!((sx, sy), (0, 0));
    }

    #[test]
    fn directions_are_in_ring_order() {
        for (i, a) in Direction::ALL.into_iter().enumerate() {
            let b = Direction::ALL[(i + 1) % 6];
            let (ha, hb) = (Hex::new(0, 0).neighbor(a), Hex::new(0, 0).neighbor(b));
            assert_eq!(ha.distance(hb), 1, "{a:?} and {b:?} must be adjacent");
        }
    }
}
