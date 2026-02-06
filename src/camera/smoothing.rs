use bevy::prelude::*;

use super::{FpsCamera, PitchAngle};

/// Applies view punch to the camera rotation
pub fn apply_view_punch_rotation(
    camera_query: Query<&FpsCamera>,
    mut pitch_query: Query<(&mut Transform, &PitchAngle), Without<FpsCamera>>,
) {
    let Ok(camera) = camera_query.single() else {
        return;
    };

    for (mut transform, pitch_angle) in &mut pitch_query {
        // Apply view punch as additional pitch rotation
        let total_pitch = pitch_angle.0 - camera.view_punch;
        transform.rotation = Quat::from_rotation_x(total_pitch);
    }
}
