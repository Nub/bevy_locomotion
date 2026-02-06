use bevy::prelude::*;

use super::effects::LedgeClimbBob;
use super::{FpsCamera, PitchAngle};

/// Applies view punch and ledge climb bob to the camera rotation
pub fn apply_view_punch_rotation(
    camera_query: Query<&FpsCamera>,
    mut pitch_query: Query<
        (&mut Transform, &PitchAngle, Option<&LedgeClimbBob>),
        Without<FpsCamera>,
    >,
) {
    let Ok(camera) = camera_query.single() else {
        return;
    };

    for (mut transform, pitch_angle, climb_bob) in &mut pitch_query {
        let mut total_pitch = pitch_angle.0 - camera.view_punch;
        let mut roll = 0.0;

        // Add ledge climb bob: pitch dip + roll to one side
        if let Some(bob) = climb_bob {
            let t = (bob.elapsed / bob.duration).clamp(0.0, 1.0);
            let wave = (t * std::f32::consts::PI).sin();
            total_pitch += wave * -0.15;
            roll = wave * 0.08 * bob.roll_sign;
        }

        transform.rotation =
            Quat::from_rotation_x(total_pitch) * Quat::from_rotation_z(roll);
    }
}
