use std::collections::HashSet;

use std::time::Instant;

pub struct GameOfLife {
    pub cells: HashSet<(u32, u32)>,
    pub width: u32,
    pub height: u32,
    pub generation: u32,
    is_dead: bool,
    death_time: Option<Instant>,
}

impl GameOfLife {
    pub fn new(width: u32, height: u32) -> Self {
        let mut gol = Self {
            cells: HashSet::new(),
            width,
            height,
            generation: 0,
            is_dead: false,
            death_time: None,
        };
        gol.randomize();
        gol
    }

    pub fn randomize(&mut self) {
        self.cells.clear();
        for y in 0..self.height {
            for x in 0..self.width {
                if rand_simple(x, y, self.generation) {
                    self.cells.insert((x, y));
                }
            }
        }
        self.generation = 0;
        self.is_dead = false;
        self.death_time = None;
    }

    pub fn step(&mut self) {
        // Check if we're in death state and need to revive
        if self.is_dead {
            if let Some(death_time) = self.death_time {
                if death_time.elapsed().as_secs() >= 10 {
                    self.randomize(); // Revive after 10 seconds
                }
            }
            return; // Don't process game logic while dead
        }

        let mut new_cells = HashSet::new();

        for y in 0..self.height {
            for x in 0..self.width {
                let neighbors = self.count_neighbors(x, y);
                let alive = self.cells.contains(&(x, y));

                if neighbors == 3 || (alive && neighbors == 2) {
                    new_cells.insert((x, y));
                }
            }
        }

        self.cells = new_cells;
        self.generation += 1;

        // Check if all cells died
        if self.cells.is_empty() {
            self.is_dead = true;
            self.death_time = Some(Instant::now());
        }
    }

    fn count_neighbors(&self, x: u32, y: u32) -> u32 {
        let mut count = 0;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0
                    && nx < self.width as i32
                    && ny >= 0
                    && ny < self.height as i32
                    && self.cells.contains(&(nx as u32, ny as u32))
                {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn get_cells(&self) -> &HashSet<(u32, u32)> {
        &self.cells
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn is_dead(&self) -> bool {
        self.is_dead
    }
}

fn rand_simple(x: u32, y: u32, seed: u32) -> bool {
    let n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(seed);
    let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n ^ (n >> 16)) & 1 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertical_neighbors() {
        let mut gol = GameOfLife::new(5, 5);
        gol.cells.clear();
        
        // Create vertical stack at (2, 1) and (2, 2)
        gol.cells.insert((2, 1));
        gol.cells.insert((2, 2));
        
        // Cell at (2, 1) should have 1 neighbor (the cell below at (2, 2))
        assert_eq!(gol.count_neighbors(2, 1), 1);
        
        // Cell at (2, 2) should have 1 neighbor (the cell above at (2, 1))
        assert_eq!(gol.count_neighbors(2, 2), 1);
        
        // Add a third cell above at (2, 0)
        gol.cells.insert((2, 0));
        
        // Now (2, 1) should have 2 neighbors (above and below)
        assert_eq!(gol.count_neighbors(2, 1), 2);
    }
}
