use heapless::Vec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameState {
    Playing,
    GameOver,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

impl Position {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }
}

pub struct Game {
    pub snake: Vec<Position, 64>, // Max snake length
    pub food: Position,
    pub direction: Direction,
    pub next_direction: Direction,
    pub state: GameState,
    pub score: u16,
    width: u8,
    height: u8,
    rng_state: u32, // Simple LFSR for random numbers
}

impl Game {
    pub fn new(width: u8, height: u8) -> Self {
        let mut snake = Vec::new();
        // Start snake in the middle of the screen
        let start_x = width / 2;
        let start_y = height / 2;
        
        snake.push(Position::new(start_x, start_y)).unwrap();
        snake.push(Position::new(start_x - 1, start_y)).unwrap();
        snake.push(Position::new(start_x - 2, start_y)).unwrap();

        let mut game = Self {
            snake,
            food: Position::new(0, 0),
            direction: Direction::Right,
            next_direction: Direction::Right,
            state: GameState::Playing,
            score: 0,
            width,
            height,
            rng_state: 0xACE1u32, // Seed for random number generator
        };

        game.spawn_food();
        game
    }

    pub fn reset(&mut self) {
        self.snake.clear();
        let start_x = self.width / 2;
        let start_y = self.height / 2;
        
        self.snake.push(Position::new(start_x, start_y)).unwrap();
        self.snake.push(Position::new(start_x - 1, start_y)).unwrap();
        self.snake.push(Position::new(start_x - 2, start_y)).unwrap();

        self.direction = Direction::Right;
        self.next_direction = Direction::Right;
        self.state = GameState::Playing;
        self.score = 0;
        self.spawn_food();
    }

    pub fn set_direction(&mut self, direction: Direction) {
        // Prevent the snake from going back into itself
        if direction != self.direction.opposite() {
            self.next_direction = direction;
        }
    }

    pub fn update(&mut self) {
        if self.state != GameState::Playing {
            return;
        }

        // Update direction
        self.direction = self.next_direction;

        // Calculate new head position
        let head = self.snake[0];
        let new_head = match self.direction {
            Direction::Up => Position::new(head.x, head.y.wrapping_sub(1)),
            Direction::Down => Position::new(head.x, head.y.wrapping_add(1)),
            Direction::Left => Position::new(head.x.wrapping_sub(1), head.y),
            Direction::Right => Position::new(head.x.wrapping_add(1), head.y),
        };

        // Check wall collision
        if new_head.x >= self.width || new_head.y >= self.height {
            self.state = GameState::GameOver;
            return;
        }

        // Check self collision
        for segment in &self.snake {
            if new_head.x == segment.x && new_head.y == segment.y {
                self.state = GameState::GameOver;
                return;
            }
        }

        // Check food collision
        let ate_food = new_head.x == self.food.x && new_head.y == self.food.y;

        // Add new head
        self.snake.insert(0, new_head).unwrap();

        if ate_food {
            self.score += 10;
            self.spawn_food();
        } else {
            // Remove tail if no food eaten
            self.snake.pop();
        }
    }

    fn spawn_food(&mut self) {
        loop {
            let x = self.next_random() % self.width as u32;
            let y = self.next_random() % self.height as u32;
            
            let new_food = Position::new(x as u8, y as u8);
            
            // Make sure food doesn't spawn on snake
            let mut valid = true;
            for segment in &self.snake {
                if segment.x == new_food.x && segment.y == new_food.y {
                    valid = false;
                    break;
                }
            }
            
            if valid {
                self.food = new_food;
                break;
            }
        }
    }

    // Simple LFSR random number generator
    fn next_random(&mut self) -> u32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        self.rng_state
    }
    
    pub fn width(&self) -> u8 {
        self.width
    }
    
    pub fn height(&self) -> u8 {
        self.height
    }
}