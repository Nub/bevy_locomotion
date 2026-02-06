use avian3d::prelude::*;
use bevy::prelude::*;

/// Maximum angle (in radians) that can be walked on
pub const MAX_SLOPE_ANGLE: f32 = 0.785; // ~45 degrees

/// Distance to cast for ground detection
pub const GROUND_CAST_DISTANCE: f32 = 0.1;

/// Result of a ground detection check
#[derive(Debug, Clone)]
pub struct GroundHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

/// Performs ground detection for a character
pub fn detect_ground(
    spatial_query: &SpatialQuery,
    position: Vec3,
    collider_radius: f32,
    collider_height: f32,
    world_layer: LayerMask,
) -> Option<GroundHit> {
    // Use a smaller sphere for ground detection to avoid false positives on walls
    let cast_radius = collider_radius * 0.5;
    let cast_shape = Collider::sphere(cast_radius);

    // Start the cast from the bottom of the capsule
    let capsule_bottom = position.y - collider_height / 2.0 + collider_radius;
    let cast_origin = Vec3::new(position.x, capsule_bottom, position.z);
    let cast_direction = Dir3::NEG_Y;

    // Cast distance: from bottom of capsule down a small amount
    let max_distance = cast_radius + GROUND_CAST_DISTANCE;

    let filter = SpatialQueryFilter::default().with_mask(world_layer);

    let config = ShapeCastConfig {
        max_distance,
        ..default()
    };

    if let Some(hit) = spatial_query.cast_shape(
        &cast_shape,
        cast_origin,
        Quat::IDENTITY,
        cast_direction,
        &config,
        &filter,
    ) {
        // Check if the surface is walkable (not too steep)
        let up = Vec3::Y;
        let angle = hit.normal1.angle_between(up);

        if angle <= MAX_SLOPE_ANGLE {
            return Some(GroundHit {
                point: hit.point1,
                normal: hit.normal1,
                distance: hit.distance,
            });
        }
    }

    None
}

/// Checks if a position is on walkable ground
pub fn is_on_ground(
    spatial_query: &SpatialQuery,
    position: Vec3,
    collider_radius: f32,
    collider_height: f32,
    world_layer: LayerMask,
) -> bool {
    detect_ground(spatial_query, position, collider_radius, collider_height, world_layer).is_some()
}
