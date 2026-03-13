use std::time::Instant;

pub struct GameOfLife {
    cells: Vec<bool>,  // flat grid: cells[y * width + x]
    pub width: u32,
    pub height: u32,
    pub generation: u32,
    is_dead: bool,
    death_time: Option<Instant>,
}

impl GameOfLife {
    pub fn new(width: u32, height: u32) -> Self {
        let mut gol = Self {
            cells: vec![false; (width * height) as usize],
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
        for y in 0..self.height {
            for x in 0..self.width {
                self.cells[(y * self.width + x) as usize] = rand_simple(x, y, self.generation);
            }
        }
        self.generation = 0;
        self.is_dead = false;
        self.death_time = None;
    }

    pub fn step(&mut self) {
        if self.is_dead {
            if let Some(death_time) = self.death_time {
                if death_time.elapsed().as_secs() >= 10 {
                    self.randomize();
                }
            }
            return;
        }

        let mut new_cells = vec![false; self.cells.len()];

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let neighbors = self.count_neighbors(x, y);
                let alive = self.cells[idx];

                if neighbors == 3 || (alive && neighbors == 2) {
                    new_cells[idx] = true;
                }
            }
        }

        self.cells = new_cells;
        self.generation += 1;

        if !self.cells.iter().any(|&c| c) {
            self.is_dead = true;
            self.death_time = Some(Instant::now());
        }
    }

    fn count_neighbors(&self, x: u32, y: u32) -> u32 {
        let mut count = 0;
        let w = self.width as i32;
        let h = self.height as i32;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < w && ny >= 0 && ny < h
                    && self.cells[(ny as u32 * self.width + nx as u32) as usize]
                {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn cell_alive(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.cells[(y * self.width + x) as usize]
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
        // Clear all cells
        gol.cells.iter_mut().for_each(|c| *c = false);

        // Create vertical stack at (2, 1) and (2, 2)
        gol.cells[7] = true;  // y=1, x=2: 1*5+2
        gol.cells[12] = true; // y=2, x=2: 2*5+2

        assert_eq!(gol.count_neighbors(2, 1), 1);
        assert_eq!(gol.count_neighbors(2, 2), 1);

        // Add a third cell above at (2, 0)
        gol.cells[2] = true; // y=0, x=2: 0*5+2

        assert_eq!(gol.count_neighbors(2, 1), 2);
    }

    #[test]
    fn test_cell_alive() {
        let mut gol = GameOfLife::new(5, 5);
        gol.cells.iter_mut().for_each(|c| *c = false);
        gol.cells[13] = true; // y=2, x=3: 2*5+3

        assert!(gol.cell_alive(3, 2));
        assert!(!gol.cell_alive(0, 0));
        assert!(!gol.cell_alive(10, 10)); // out of bounds
    }

    #[test]
    fn test_step_blinker() {
        // Horizontal blinker at row 2: cells (1,2), (2,2), (3,2)
        let mut gol = GameOfLife::new(5, 5);
        gol.cells.iter_mut().for_each(|c| *c = false);
        gol.cells[11] = true; // y=2, x=1: 2*5+1
        gol.cells[12] = true; // y=2, x=2: 2*5+2
        gol.cells[13] = true; // y=2, x=3: 2*5+3

        gol.step();

        // After one step, blinker should be vertical: (2,1), (2,2), (2,3)
        assert!(gol.cell_alive(2, 1));
        assert!(gol.cell_alive(2, 2));
        assert!(gol.cell_alive(2, 3));
        assert!(!gol.cell_alive(1, 2));
        assert!(!gol.cell_alive(3, 2));
    }
}
