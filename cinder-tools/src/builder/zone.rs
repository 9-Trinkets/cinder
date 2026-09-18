#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Direction::North => "North",
            Direction::South => "South",
            Direction::East => "East",
            Direction::West => "West",
            Direction::Up => "Up",
            Direction::Down => "Down",
        }
    }

    pub fn default_aliases(&self) -> Vec<String> {
        match self {
            Direction::North => vec!["n".to_string(), "north".to_string()],
            Direction::South => vec!["s".to_string(), "south".to_string()],
            Direction::East => vec!["e".to_string(), "east".to_string()],
            Direction::West => vec!["w".to_string(), "west".to_string()],
            Direction::Up => vec!["u".to_string(), "up".to_string()],
            Direction::Down => vec!["d".to_string(), "down".to_string()],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Zone {
    pub name: String,
    pub min_x: usize,
    pub max_x: usize,
    pub min_y: usize,
    pub max_y: usize,
}

impl Zone {
    pub fn new(name: impl Into<String>, min_x: usize, max_x: usize, min_y: usize, max_y: usize) -> Self {
        Self {
            name: name.into(),
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    pub fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}
