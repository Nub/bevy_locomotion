use bevy::prelude::*;
use player_controller::{CameraPlugin, PhysicsPlugin, PlayerPlugin, WorldPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "FPS Character Controller".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            PhysicsPlugin,
            PlayerPlugin,
            CameraPlugin,
            WorldPlugin,
        ))
        .run();
}
