use bevy::{prelude::*, window::{CursorGrabMode, CursorOptions, PrimaryWindow}};

use super::{effects::*, look::*, smoothing::*};

/// Plugin for FPS camera systems
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreviousGroundedState>();

        app.add_systems(Startup, setup_cursor_grab);

        app.add_systems(
            Update,
            (
                sync_camera_to_player,
                apply_mouse_look,
                update_fov,
                apply_head_bob,
                apply_view_punch,
                update_camera_height,
                apply_view_punch_rotation,
            )
                .chain(),
        );

        app.add_systems(Update, toggle_cursor_grab);
    }
}

/// Grabs and hides the cursor for FPS controls
fn setup_cursor_grab(mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor_query.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Escape releases cursor, mouse click recaptures
fn toggle_cursor_grab(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(mut cursor) = cursor_query.single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if mouse.just_pressed(MouseButton::Left) && cursor.grab_mode == CursorGrabMode::None {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}
