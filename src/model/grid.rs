use std::collections::HashSet;

pub type Position = (isize, isize);

const ODD_ADJACENT_POSITIONS: [(isize, isize); 6] = [
    (-1, 0), // upper left
    (-1, 1), // upper right
    (0, -1), // left
    (0, 1),  // right
    (1, 0),  // bottom left
    (1, 1),  // bottom right
];

const EVEN_ADJACENT_POSITIONS: [(isize, isize); 6] = [
    (-1, -1), // upper left
    (-1, 0),  // upper right
    (0, -1),  // left
    (0, 1),   // right
    (1, -1),  // bottom left
    (1, 0),   // bottom right
];

pub struct HexGrid {
    pub rows: usize,
    pub cols: usize,
}

impl HexGrid {
    pub fn new(rows: usize, cols: usize) -> HexGrid {
        HexGrid {
            rows: rows,
            cols: cols,
        }
    }

    // Positions adjacent to specified position
    pub fn adjacent(&self, pos: Position) -> Vec<Position> {
        let shifts = if pos.0 % 2 == 0 {
            EVEN_ADJACENT_POSITIONS
        } else {
            ODD_ADJACENT_POSITIONS
        };
        shifts
            .iter()
            // Shift positions
            .map(|s| (pos.0 + s.0, pos.1 + s.1))
            // Check w/in grid bounds
            .filter(|p| {
                p.0 >= 0 && p.0 < (self.rows as isize) && p.1 >= 0 && p.1 < (self.cols as isize)
            })
            .map(|p| (p.0, p.1))
            .collect()
    }

    // Positions within a radius of the specified position
    pub fn radius(&self, pos: Position, r: usize) -> Vec<Position> {
        let mut neighbs = HashSet::new();
        let mut next = vec![pos];
        for _ in 0..r {
            let adj: Vec<Position> = next.iter().flat_map(|&p| self.adjacent(p)).collect();
            neighbs.extend(adj.to_vec());
            next = adj;
        }
        neighbs.into_iter().collect()
    }

    // Get a Vec of all Positions
    pub fn positions(&self) -> Vec<Position> {
        (0..self.rows)
            .flat_map(|r| (0..self.cols)
                      .map(move |c| (r as isize, c as isize)))
            .collect()
    }
}

// 2D euclidean distance
pub fn euclidean_dist(a: &Position, b: &Position) -> f32 {
    (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f32).sqrt()
}

pub fn hexagon_dist(a: &Position, b: &Position) -> usize {
    let x0 = a.0 - b.0/2;
    let y0 = b.0;
    let x1 = a.1 - b.1/2;
    let y1 = b.1;
    let dx = x1 - x0;
    let dy = y1 - y0;
    dx.abs().max(dy.abs().max((dx+dy).abs())) as usize
}

