use bevy::prelude::*;

use super::{effects::*, look::*, smoothing::*};

/// Plugin for FPS camera systems
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreviousGroundedState>();

        app.add_systems(
            Update,
            (
                sync_camera_to_player,
                apply_mouse_look,
                update_fov,
                apply_head_bob,
                apply_ledge_climb_bob,
                apply_view_punch,
                update_camera_height,
                apply_ledge_grab_bounce,
                apply_ledge_shuffle_bob,
                apply_view_punch_rotation,
            )
                .chain(),
        );

    }
}
