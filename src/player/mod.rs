mod crouch;
pub mod input;
mod jump;
mod movement;
mod plugin;
mod state;

pub use input::{LookInput, MoveInput};
pub use plugin::PlayerPlugin;
pub use state::*;
