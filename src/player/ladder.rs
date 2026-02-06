use avian3d::prelude::*;
use bevy::prelude::*;

use super::input::{JumpPressed, MoveInput};
use super::state::*;

/// Marker component for world geometry that acts as a climbable ladder.
///
/// Ladder entities should use `Sensor` colliders on `GameLayer::Trigger` so
/// the player can overlap them.
#[derive(Component)]
pub struct Ladder;

/// Detects when a player enters a ladder volume and starts climbing.
///
/// The player must be pressing up (`move_input.y > 0.5`) while overlapping
/// a `Ladder` entity.
pub fn detect_ladder(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    query: Query<
        (Entity, &Transform, &PlayerConfig, &MoveInput),
        (With<Player>, Without<OnLadder>),
    >,
    ladder_query: Query<&Transform, With<Ladder>>,
) {
    for (entity, transform, config, move_input) in &query {
        // Must be pressing up to grab ladder
        if move_input.y < 0.5 {
            continue;
        }

        let capsule_height = config.stand_height - config.radius * 2.0;
        let shape = Collider::capsule(config.radius, capsule_height);
        let shape_pos = transform.translation;
        let shape_rot = transform.rotation;

        let filter = SpatialQueryFilter::default()
            .with_mask(config.collision_mask);

        let intersections = spatial_query.shape_intersections(
            &shape,
            shape_pos,
            shape_rot,
            &filter,
        );

        for hit_entity in &intersections {
            let Ok(ladder_transform) = ladder_query.get(*hit_entity) else {
                continue;
            };

            // Compute outward normal: horizontal direction from ladder center to player
            let to_player = transform.translation - ladder_transform.translation;
            let horizontal = Vec3::new(to_player.x, 0.0, to_player.z);
            let outward_normal = horizontal.normalize_or_zero();

            if outward_normal.length_squared() < 0.01 {
                continue;
            }

            commands.entity(entity).insert(OnLadder { outward_normal });
            break;
        }
    }
}

/// Applies ladder movement: climb up/down with move input, jump to dismount.
///
/// Removes `OnLadder` when the player jumps off or leaves the ladder volume.
pub fn apply_ladder_movement(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut query: Query<
        (
            Entity,
            &Transform,
            &PlayerConfig,
            &mut PlayerVelocity,
            &OnLadder,
            &MoveInput,
            &mut JumpPressed,
        ),
        With<Player>,
    >,
    ladder_query: Query<(), With<Ladder>>,
) {
    for (entity, transform, config, mut velocity, on_ladder, move_input, mut jump_pressed) in
        &mut query
    {
        // Check still overlapping a ladder
        let capsule_height = config.stand_height - config.radius * 2.0;
        let shape = Collider::capsule(config.radius, capsule_height);

        let filter = SpatialQueryFilter::default()
            .with_mask(config.collision_mask);

        let intersections = spatial_query.shape_intersections(
            &shape,
            transform.translation,
            transform.rotation,
            &filter,
        );

        let still_on_ladder = intersections
            .iter()
            .any(|e| ladder_query.get(*e).is_ok());

        if !still_on_ladder {
            commands.entity(entity).remove::<OnLadder>();
            continue;
        }

        // Jump to dismount
        if jump_pressed.0 {
            jump_pressed.0 = false;
            velocity.0 = on_ladder.outward_normal * config.jump_velocity * 0.4
                + Vec3::Y * config.jump_velocity;
            commands.entity(entity).remove::<OnLadder>();
            continue;
        }

        // Climb: vertical movement from input Y
        velocity.0 = Vec3::Y * move_input.y * config.ladder_climb_speed;
    }
}
