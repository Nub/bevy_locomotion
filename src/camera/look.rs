use bevy::prelude::*;

use crate::player::{LookInput, Player};

/// Marker for the yaw (horizontal rotation) entity
#[derive(Component)]
pub struct CameraYaw;

/// Marker for the pitch (vertical rotation) entity
#[derive(Component)]
pub struct CameraPitch;

/// Camera configuration
#[derive(Component, Clone)]
pub struct CameraConfig {
    /// Mouse sensitivity
    pub sensitivity: f32,
    /// Maximum pitch angle (looking up)
    pub max_pitch: f32,
    /// Minimum pitch angle (looking down)
    pub min_pitch: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            sensitivity: 0.003,
            max_pitch: 89.0_f32.to_radians(),
            min_pitch: -89.0_f32.to_radians(),
        }
    }
}

/// Current pitch angle in radians
#[derive(Component, Default, Deref, DerefMut)]
pub struct PitchAngle(pub f32);

/// Applies mouse look rotation to camera
pub fn apply_mouse_look(
    player_query: Query<&LookInput, With<Player>>,
    mut yaw_query: Query<&mut Transform, (With<CameraYaw>, Without<CameraPitch>)>,
    mut pitch_query: Query<(&mut Transform, &mut PitchAngle, &CameraConfig), With<CameraPitch>>,
) {
    let Ok(look_input) = player_query.single() else {
        return;
    };

    // Apply yaw (horizontal rotation)
    if let Ok(mut yaw_transform) = yaw_query.single_mut() {
        yaw_transform.rotate_y(-look_input.x * 0.003); // Use default sensitivity inline
    }

    // Apply pitch (vertical rotation)
    if let Ok((mut pitch_transform, mut pitch_angle, config)) = pitch_query.single_mut() {
        pitch_angle.0 -= look_input.y * config.sensitivity;
        pitch_angle.0 = pitch_angle.0.clamp(config.min_pitch, config.max_pitch);

        pitch_transform.rotation = Quat::from_rotation_x(pitch_angle.0);
    }
}

/// Syncs the camera yaw position to follow the player
pub fn sync_camera_to_player(
    player_query: Query<&Transform, With<Player>>,
    mut yaw_query: Query<&mut Transform, (With<CameraYaw>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    if let Ok(mut yaw_transform) = yaw_query.single_mut() {
        yaw_transform.translation = player_transform.translation;
    }
}
