use avian3d::prelude::*;
use bevy::prelude::*;

use super::input::MoveInput;
use super::state::*;
use crate::camera::CameraYaw;
use crate::physics::GameLayer;

/// Updates grounded state via raycast
pub fn update_grounded_state(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut query: Query<(
        Entity,
        &Transform,
        &PlayerConfig,
        &PlayerVelocity,
        &mut CoyoteTime,
        &mut AirTime,
        Option<&Grounded>,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, transform, config, player_vel, mut coyote, mut air_time, was_grounded) in &mut query {
        // Raycast from center of capsule downward
        let ray_origin = transform.translation;
        let ray_dir = Dir3::NEG_Y;
        let ground_check_dist = config.stand_height / 2.0 + 0.1;

        let filter = SpatialQueryFilter::default()
            .with_mask(GameLayer::World);

        let hit = spatial_query.cast_ray(
            ray_origin,
            ray_dir,
            ground_check_dist,
            true,
            &filter,
        );

        let is_grounded = hit.as_ref()
            .is_some_and(|h| h.distance < ground_check_dist && player_vel.y < 1.0);

        if is_grounded {
            let normal = hit.unwrap().normal;
            commands.entity(entity).insert(GroundNormal(normal));
            if was_grounded.is_none() {
                commands.entity(entity).insert(Grounded);
            }
            coyote.timer = 0.0;
            air_time.duration = 0.0;
        } else {
            commands.entity(entity).remove::<GroundNormal>();
            if was_grounded.is_some() {
                commands.entity(entity).remove::<Grounded>();
            }
            coyote.timer += dt;
            air_time.duration += dt;

            // If vertical velocity is near zero while airborne, the player is
            // likely resting on an edge the center ray missed — keep coyote
            // alive so they can still jump and move.
            if player_vel.y.abs() < 0.5 {
                coyote.timer = 0.0;
            }
        }
    }
}

/// Applies ground movement - sets horizontal velocity
pub fn ground_movement(
    mut query: Query<
        (
            &MoveInput,
            &PlayerConfig,
            &mut PlayerVelocity,
            Has<Sprinting>,
            Has<Crouching>,
        ),
        (With<Grounded>, Without<Sliding>),
    >,
    yaw_query: Query<&Transform, With<CameraYaw>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    let Ok(yaw_transform) = yaw_query.single() else {
        return;
    };

    for (input, config, mut velocity, sprinting, crouching) in &mut query {
        let forward = yaw_transform.forward().as_vec3();
        let right = yaw_transform.right().as_vec3();

        // Flatten to horizontal
        let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        let move_dir = (forward * input.y + right * input.x).normalize_or_zero();
        let target_speed = if crouching {
            config.crouch_speed
        } else if sprinting {
            config.sprint_speed
        } else {
            config.walk_speed
        };

        let target = move_dir * target_speed;
        let current = Vec3::new(velocity.x, 0.0, velocity.z);

        let accel = if input.length_squared() > 0.01 {
            config.ground_accel
        } else {
            config.ground_friction
        };

        let new_vel = current.move_towards(target, accel * dt);
        velocity.x = new_vel.x;
        velocity.z = new_vel.z;
    }
}

/// Applies air movement with reduced control
pub fn air_movement(
    mut query: Query<
        (&MoveInput, &PlayerConfig, &mut PlayerVelocity),
        (Without<Grounded>, Without<LedgeGrabbing>, Without<LedgeClimbing>),
    >,
    yaw_query: Query<&Transform, With<CameraYaw>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    let Ok(yaw_transform) = yaw_query.single() else {
        return;
    };

    for (input, config, mut velocity) in &mut query {
        if input.length_squared() < 0.01 {
            continue;
        }

        let forward = yaw_transform.forward().as_vec3();
        let right = yaw_transform.right().as_vec3();
        let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        let move_dir = (forward * input.y + right * input.x).normalize_or_zero();

        // Use ground accel when resting on an edge (near-zero vertical velocity)
        let accel = if velocity.y.abs() < 0.5 {
            config.ground_accel
        } else {
            config.air_accel
        };

        let current_speed = velocity.dot(move_dir);
        let add_speed = (config.walk_speed - current_speed).max(0.0);
        let accel_speed = (accel * dt).min(add_speed);

        velocity.x += move_dir.x * accel_speed;
        velocity.z += move_dir.z * accel_speed;
    }
}

/// Applies gravity when not grounded
pub fn apply_gravity(
    mut query: Query<&mut PlayerVelocity, (Without<Grounded>, Without<LedgeGrabbing>, Without<LedgeClimbing>)>,
    gravity: Res<Gravity>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for mut velocity in &mut query {
        velocity.0 += gravity.0 * dt;
    }
}

/// Syncs PlayerVelocity to Avian's LinearVelocity, projecting onto ground surface when grounded
pub fn apply_velocity(
    mut query: Query<
        (&mut PlayerVelocity, &PlayerConfig, &mut LinearVelocity, Option<&Grounded>, Option<&GroundNormal>),
        With<Player>,
    >,
) {
    for (mut player_vel, config, mut lin_vel, grounded, ground_normal) in &mut query {
        // Clamp horizontal speed
        if config.max_horizontal_speed > 0.0 {
            let h_speed = Vec2::new(player_vel.x, player_vel.z).length();
            if h_speed > config.max_horizontal_speed {
                let scale = config.max_horizontal_speed / h_speed;
                player_vel.x *= scale;
                player_vel.z *= scale;
            }
        }

        if grounded.is_some() {
            let horizontal = Vec3::new(player_vel.x, 0.0, player_vel.z);

            if let Some(GroundNormal(normal)) = ground_normal {
                // Project horizontal velocity onto slope surface to maintain speed on inclines
                let projected = horizontal - *normal * horizontal.dot(*normal);
                let horizontal_speed = horizontal.length();

                if horizontal_speed > 0.01 {
                    // Rescale so the horizontal component of projected velocity matches desired speed.
                    // This preserves full move speed on slopes instead of losing it to collision.
                    let proj_horiz = Vec2::new(projected.x, projected.z).length();
                    let scale = if proj_horiz > 0.001 {
                        horizontal_speed / proj_horiz
                    } else {
                        1.0
                    };
                    let slope_vel = projected * scale;
                    lin_vel.x = slope_vel.x;
                    lin_vel.y = (player_vel.y + slope_vel.y).min(slope_vel.y);
                    lin_vel.z = slope_vel.z;
                } else {
                    lin_vel.x = 0.0;
                    lin_vel.z = 0.0;
                    lin_vel.y = player_vel.y.min(-0.5);
                }
            } else {
                lin_vel.x = player_vel.x;
                lin_vel.z = player_vel.z;
                lin_vel.y = player_vel.y.min(-0.5);
            }
        } else {
            lin_vel.x = player_vel.x;
            lin_vel.z = player_vel.z;
            lin_vel.y = player_vel.y;
        }
    }
}

/// Updates sprint state and sprint grace timer
pub fn update_sprint_state(
    mut commands: Commands,
    mut query: Query<
        (Entity, &super::input::SprintInput, &mut SprintGrace, Has<Grounded>, Has<Crouching>),
        With<Player>,
    >,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (entity, sprint_input, mut grace, grounded, crouching) in &mut query {
        if sprint_input.0 && grounded && !crouching {
            commands.entity(entity).insert(Sprinting);
            grace.timer = 0.0;
        } else {
            commands.entity(entity).remove::<Sprinting>();
            grace.timer += dt;
        }
    }
}
