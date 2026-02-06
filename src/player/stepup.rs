use avian3d::prelude::*;
use bevy::prelude::*;

use super::audio::PlayerAudioMessage;
use super::state::*;

/// Auto-steps the player over small obstacles (stairs, curbs) when grounded and moving.
///
/// Uses a three-ray approach:
/// 1. **Foot ray** (forward from ankle): must HIT — obstacle exists
/// 2. **Step ray** (forward from step height): must MISS — space above obstacle
/// 3. **Surface ray** (downward at obstacle distance): must HIT with upward normal — step surface
pub fn apply_step_up(
    spatial_query: SpatialQuery,
    mut query: Query<
        (&mut Transform, &PlayerConfig, &PlayerVelocity),
        With<Grounded>,
    >,
    mut writer: MessageWriter<PlayerAudioMessage>,
) {
    for (mut transform, config, velocity) in &mut query {
        let filter = SpatialQueryFilter::default().with_mask(config.world_layer);
        let h_vel = Vec3::new(velocity.x, 0.0, velocity.z);
        if h_vel.length_squared() < 0.25 {
            continue;
        }

        let forward_dir = match Dir3::new(h_vel.normalize()) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let half_height = config.stand_height / 2.0;
        let center = transform.translation;
        let probe_dist = config.radius + 0.15;

        // Ray 1: foot height (ankle) — must HIT (obstacle exists)
        let foot_origin = center + Vec3::Y * (-half_height + 0.05);
        let foot_hit = spatial_query.cast_ray(
            foot_origin,
            forward_dir,
            probe_dist,
            true,
            &filter,
        );
        let Some(foot_hit) = foot_hit else {
            continue;
        };

        // Ray 2: step height — must MISS (space above obstacle)
        let step_origin = center + Vec3::Y * (-half_height + config.step_up_height);
        let step_hit = spatial_query.cast_ray(
            step_origin,
            forward_dir,
            probe_dist,
            true,
            &filter,
        );
        if step_hit.is_some() {
            continue;
        }

        // Ray 3: downward from step height at obstacle distance — must HIT with upward normal
        let obstacle_point = foot_origin + h_vel.normalize() * foot_hit.distance;
        let surface_origin = Vec3::new(
            obstacle_point.x,
            center.y + (-half_height + config.step_up_height),
            obstacle_point.z,
        );
        let surface_hit = spatial_query.cast_ray(
            surface_origin,
            Dir3::NEG_Y,
            config.step_up_height,
            true,
            &filter,
        );
        let Some(surface_hit) = surface_hit else {
            continue;
        };

        if surface_hit.normal.dot(Vec3::Y) < 0.7 {
            continue;
        }

        let surface_y = surface_origin.y - surface_hit.distance;
        transform.translation.y = surface_y + half_height;

        writer.write(PlayerAudioMessage::SteppedUp);
    }
}
