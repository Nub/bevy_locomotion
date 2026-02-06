pub mod camera;
pub mod physics;
pub mod player;

pub use camera::CameraPlugin;
pub use physics::PhysicsPlugin;
pub use player::PlayerPlugin;

use bevy::prelude::*;

/// Unified plugin that adds physics, player controller, and camera systems.
pub struct BevyLocomotionPlugin;

impl Plugin for BevyLocomotionPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<PhysicsPlugin>() {
            app.add_plugins(PhysicsPlugin);
        }
        if !app.is_plugin_added::<PlayerPlugin>() {
            app.add_plugins(PlayerPlugin);
        }
        if !app.is_plugin_added::<CameraPlugin>() {
            app.add_plugins(CameraPlugin);
        }
    }
}

pub mod prelude {
    pub use crate::camera::{CameraConfig, CameraPlugin, FpsCamera};
    pub use crate::physics::{GameLayer, PhysicsPlugin};
    pub use crate::player::{
        spawn_player, Crouching, ForceSlide, ForcedSliding, Grounded, Ladder, LedgeClimbing,
        LedgeGrabbable, LedgeGrabbing, OnLadder, Player, PlayerAudioMessage, PlayerConfig,
        PlayerPlugin, PlayerVelocity, Sliding, Sprinting,
    };
    pub use crate::BevyLocomotionPlugin;
}
