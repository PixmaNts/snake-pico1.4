use crate::game::{Game, GameState};
use crate::traits::{GameInput, GamePlatform, GameRenderer, InputEvent};

pub struct GameEngine<I, P, R>
where
    I: GameInput,
    P: GamePlatform,
    R: GameRenderer,
{
    input: I,
    platform: P,
    renderer: R,
    game: Game,
    target_frame_time_ms: u32,
}

impl<I, P, R> GameEngine<I, P, R>
where
    I: GameInput,
    P: GamePlatform,
    R: GameRenderer,
{
    pub fn new(input: I, platform: P, renderer: R, grid_width: u8, grid_height: u8) -> Self {
        Self {
            input,
            platform,
            renderer,
            game: Game::new(grid_width, grid_height),
            target_frame_time_ms: 150, // Default to ~7 FPS
        }
    }

    #[allow(dead_code)]
    pub fn set_frame_rate(&mut self, fps: u32) {
        self.target_frame_time_ms = 1000 / fps;
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        loop {
            let frame_start = self.platform.current_time_ms();

            // Handle input
            match self.input.read_input().await {
                Ok(InputEvent::Direction(dir)) => {
                    if self.game.state == GameState::Playing {
                        self.game.set_direction(dir);
                    }
                }
                Ok(InputEvent::ButtonA) => {
                    if self.game.state == GameState::GameOver {
                        self.game.reset();
                    }
                }
                Ok(InputEvent::ButtonB) => {
                    // Reserved for future use (pause, menu, etc.)
                }
                Ok(InputEvent::None) => {}
                Err(_) => {
                    // Handle input error gracefully
                    continue;
                }
            }

            // Update game logic
            if self.game.state == GameState::Playing {
                self.game.update();
            }

            // Render game
            if let Err(_) = self.renderer.render_game(
                &self.game.snake,
                &self.game.food,
                self.game.score,
                self.game.state,
                self.game.width(),
                self.game.height(),
            ) {
                // Handle render error by continuing
                continue;
            }

            // Frame timing
            let frame_time = self.platform.current_time_ms() - frame_start;
            if frame_time < self.target_frame_time_ms {
                self.platform
                    .delay_ms(self.target_frame_time_ms - frame_time)
                    .await;
            }
        }
    }

    #[allow(dead_code)]
    pub fn game(&self) -> &Game {
        &self.game
    }

    #[allow(dead_code)]
    pub fn game_mut(&mut self) -> &mut Game {
        &mut self.game
    }
}
