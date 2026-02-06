use avian3d::prelude::*;
use bevy::prelude::*;

use super::input::CrouchInput;
use super::state::*;
use crate::physics::GameLayer;

/// Updates crouch state and handles slide initiation
pub fn update_crouch_state(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &CrouchInput,
        &PlayerConfig,
        &PlayerVelocity,
        &Transform,
        &SprintGrace,
        Has<Grounded>,
        Has<Sprinting>,
        Has<Crouching>,
        Option<&Sliding>,
        Has<PendingSlide>,
    )>,
    spatial_query: SpatialQuery,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();

    for (entity, crouch_input, config, velocity, transform, sprint_grace, grounded, sprinting, crouching, sliding, pending_slide) in
        &mut query
    {
        if crouch_input.0 {
            // Already sliding - let apply_slide manage it
            if sliding.is_some() {
                continue;
            }

            let horizontal_vel = Vec3::new(velocity.x, 0.0, velocity.z);
            let horizontal_speed = horizontal_vel.length();

            // Landed with a pending slide from air
            if pending_slide && grounded {
                commands.entity(entity).remove::<PendingSlide>();
                if horizontal_speed > 0.5 {
                    let dir = horizontal_vel.normalize_or_zero();
                    commands.entity(entity).insert((
                        Crouching,
                        Sliding {
                            direction: dir,
                            start_time: current_time,
                            initial_speed: horizontal_speed * config.slide_boost,
                        },
                    ));
                    commands.entity(entity).remove::<Sprinting>();
                    continue;
                }
            }

            // Buffer slide if pressing crouch in the air with speed
            if !grounded && !crouching && horizontal_speed > config.min_slide_speed {
                commands.entity(entity).insert((Crouching, PendingSlide));
                continue;
            }

            // Check if we should start sliding (ground initiation)
            let in_grace = sprint_grace.timer < config.sprint_slide_grace;

            let slide_initiate = if sprinting && horizontal_speed >= config.min_slide_speed {
                // Active sprint slide
                Some((horizontal_vel.normalize_or_zero(), horizontal_speed))
            } else if !crouching && grounded && in_grace && horizontal_speed > 0.5 {
                // Grace window slide
                let dir = horizontal_vel.normalize_or_zero();
                Some((dir, config.sprint_speed))
            } else {
                None
            };

            if let Some((slide_dir, slide_speed)) = slide_initiate {
                if !crouching && grounded {
                    commands.entity(entity).insert((
                        Crouching,
                        Sliding {
                            direction: slide_dir,
                            start_time: current_time,
                            initial_speed: slide_speed * config.slide_boost,
                        },
                    ));
                    commands.entity(entity).remove::<Sprinting>();
                }
            } else if !crouching {
                // Regular crouch
                commands.entity(entity).insert(Crouching);
            }
        } else {
            commands.entity(entity).remove::<PendingSlide>();
            if crouching {
                // Try to stand up - check if there's room
                if can_stand_up(&spatial_query, transform.translation, config) {
                    commands.entity(entity).remove::<Crouching>();
                    commands.entity(entity).remove::<Sliding>();
                }
            }
        }
    }
}

/// Applies slide movement
pub fn apply_slide(
    mut commands: Commands,
    mut query: Query<(Entity, &PlayerConfig, &mut PlayerVelocity, &Sliding)>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();

    for (entity, config, mut velocity, sliding) in &mut query {
        let elapsed = current_time - sliding.start_time;

        if elapsed >= config.slide_duration {
            // End slide
            commands.entity(entity).remove::<Sliding>();
            continue;
        }

        // Gradual deceleration curve: higher slide_friction = more speed retained early
        let t = elapsed / config.slide_duration;
        let speed = sliding.initial_speed * (1.0 - t.powf(config.slide_friction));

        // Override horizontal velocity with slide
        velocity.x = sliding.direction.x * speed;
        velocity.z = sliding.direction.z * speed;
    }
}

/// Checks if there's room for the player to stand up
fn can_stand_up(spatial_query: &SpatialQuery, position: Vec3, config: &PlayerConfig) -> bool {
    let height_diff = config.stand_height - config.crouch_height;
    let check_shape = Collider::capsule(config.radius * 0.9, height_diff);

    let filter = SpatialQueryFilter::default().with_mask(GameLayer::World);

    // Check space above the crouched player
    let check_pos = position + Vec3::Y * (config.crouch_height / 2.0 + height_diff / 2.0);

    let cast_config = ShapeCastConfig {
        max_distance: 0.01,
        ..default()
    };

    spatial_query
        .cast_shape(&check_shape, check_pos, Quat::IDENTITY, Dir3::Y, &cast_config, &filter)
        .is_none()
}

/// Updates collider height based on crouch state
pub fn update_collider_height(
    mut query: Query<(&PlayerConfig, &mut Collider, Has<Crouching>), With<Player>>,
) {
    for (config, mut collider, crouching) in &mut query {
        let target_height = if crouching {
            config.crouch_height
        } else {
            config.stand_height
        };

        // Create new capsule with target height
        let capsule_height = target_height - config.radius * 2.0;
        *collider = Collider::capsule(config.radius, capsule_height.max(0.1));
    }
}
