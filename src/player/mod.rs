pub mod audio;
mod crouch;
mod forceslide;
pub mod input;
mod jump;
mod ladder;
mod ledge;
mod movement;
pub(crate) mod plugin;
mod state;
mod stepup;

pub use audio::PlayerAudioMessage;
pub use forceslide::ForceSlide;
pub use input::{LookInput, MoveInput};
pub use ladder::Ladder;
pub use ledge::LedgeGrabbable;
/// The curve `apply_gravity` flies, exposed so a host can assert that the arc it
/// tunes for is the arc the controller actually applies.
pub use movement::gravity_scale;
pub use plugin::{spawn_player, PlayerPlugin};
pub use state::*;
