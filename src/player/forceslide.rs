use avian3d::prelude::*;
use bevy::prelude::*;

use super::state::*;

/// Marker component for world geometry that forces the player to slide downhill.
#[derive(Component)]
pub struct ForceSlide;

/// Detects when a grounded player is standing on a `ForceSlide` surface and
/// initiates forced sliding in the downhill direction.
pub fn detect_forced_slide(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    query: Query<
        (Entity, &Transform, &PlayerConfig),
        (With<Player>, With<Grounded>, Without<ForcedSliding>),
    >,
    surface_query: Query<(), With<ForceSlide>>,
    gravity: Res<Gravity>,
) {
    for (entity, transform, config) in &query {
        let filter = SpatialQueryFilter::default().with_mask(config.world_layer);
        let ground_check_dist = config.stand_height / 2.0 + 0.2;

        let hit = spatial_query.cast_ray(
            transform.translation,
            Dir3::NEG_Y,
            ground_check_dist,
            true,
            &filter,
        );

        let Some(hit) = hit else { continue };

        // Surface must have ForceSlide marker
        if surface_query.get(hit.entity).is_err() {
            continue;
        }

        let normal = hit.normal;

        // Skip flat surfaces — no sliding needed
        if normal.dot(Vec3::Y) > 0.99 {
            continue;
        }

        // Compute downhill direction: project gravity onto the slope surface
        let gravity_vec = gravity.0;
        let projected = gravity_vec - normal * gravity_vec.dot(normal);
        let direction = projected.normalize_or_zero();

        if direction.length_squared() < 0.01 {
            continue;
        }

        commands.entity(entity).insert(ForcedSliding {
            direction,
            surface_normal: normal,
        });

        // Remove voluntary sliding to avoid conflicts
        commands.entity(entity).remove::<Sliding>();
    }
}

/// Accelerates the player in the downhill direction while on a `ForceSlide` surface.
/// Removes `ForcedSliding` when the player leaves the surface.
pub fn apply_forced_slide(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut query: Query<
        (Entity, &Transform, &PlayerConfig, &mut PlayerVelocity, &ForcedSliding),
        With<Player>,
    >,
    surface_query: Query<(), With<ForceSlide>>,
    gravity: Res<Gravity>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, transform, config, mut velocity, forced) in &mut query {
        let filter = SpatialQueryFilter::default().with_mask(config.world_layer);
        let ground_check_dist = config.stand_height / 2.0 + 0.2;

        let hit = spatial_query.cast_ray(
            transform.translation,
            Dir3::NEG_Y,
            ground_check_dist,
            true,
            &filter,
        );

        // Check we're still on a ForceSlide surface
        let still_on = hit
            .as_ref()
            .is_some_and(|h| surface_query.get(h.entity).is_ok());

        if !still_on {
            commands.entity(entity).remove::<ForcedSliding>();
            continue;
        }

        // Accelerate downhill: stronger on steeper slopes
        let normal = forced.surface_normal;
        let gravity_magnitude = gravity.0.length();
        let slope_accel = gravity_magnitude * (1.0 - normal.dot(Vec3::Y));

        velocity.0 += forced.direction * slope_accel * dt;
    }
}
