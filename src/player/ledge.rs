use avian3d::prelude::*;
use bevy::prelude::*;
use rand::prelude::*;

use super::input::{CrouchInput, JumpPressed, MoveInput};
use super::state::*;
use crate::camera::{CameraPitch, CameraYaw, LedgeClimbBob, LedgeGrabBounce, LedgeShuffleBob};
use crate::physics::GameLayer;

/// Detects ledge grabs using a three-ray approach.
///
/// When the player is airborne and moving toward a wall:
/// 1. Ray 1 (head height, forward) must MISS (open air above ledge)
/// 2. Ray 2 (chest height, forward) must HIT (wall exists)
/// 3. Ray 3 (downward from above wall hit) must HIT with upward normal (ledge surface)
pub fn detect_ledge_grab(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut query: Query<
        (
            Entity,
            &Transform,
            &PlayerConfig,
            &PlayerVelocity,
            &mut LedgeCooldown,
            &mut JumpPressed,
        ),
        (Without<Grounded>, Without<LedgeGrabbing>),
    >,
    pitch_query: Query<Entity, With<CameraPitch>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let filter = SpatialQueryFilter::default().with_mask(GameLayer::World);

    for (entity, transform, config, velocity, mut cooldown, mut jump_pressed) in &mut query {
        cooldown.timer += dt;
        if cooldown.timer < config.ledge_cooldown {
            continue;
        }

        // Only grab when jump is pressed
        if !jump_pressed.0 {
            continue;
        }

        // Must be falling (unless ascending grabs are enabled)
        if !config.ledge_grab_ascending && velocity.y > 0.0 {
            continue;
        }

        // Reject if falling too fast
        if config.ledge_grab_max_fall_speed > 0.0 && velocity.y < -config.ledge_grab_max_fall_speed
        {
            continue;
        }

        // Need horizontal movement to determine probe direction
        let h_vel = Vec3::new(velocity.x, 0.0, velocity.z);
        if h_vel.length_squared() < 0.1 {
            continue;
        }

        let forward_dir = match Dir3::new(h_vel.normalize()) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let half_height = config.stand_height / 2.0;
        let center = transform.translation;
        let probe_dist = config.radius + config.ledge_detect_reach;

        // Ray 1: head height — must MISS (open air above ledge)
        let ray1_origin = center + Vec3::Y * half_height;
        let ray1_hit = spatial_query.cast_ray(
            ray1_origin,
            forward_dir,
            probe_dist,
            true,
            &filter,
        );
        if ray1_hit.is_some() {
            continue;
        }

        // Ray 2: chest height — must HIT (wall exists)
        let ray2_origin = center + Vec3::Y * (half_height * 0.3);
        let ray2_hit = spatial_query.cast_ray(
            ray2_origin,
            forward_dir,
            probe_dist,
            true,
            &filter,
        );
        let Some(wall_hit) = ray2_hit else {
            continue;
        };

        // Ray 3: downward from above the wall hit point — must HIT with upward normal
        let wall_point = ray2_origin + h_vel.normalize() * wall_hit.distance;
        let ray3_origin = Vec3::new(wall_point.x, ray1_origin.y + 0.3, wall_point.z);
        let ray3_hit = spatial_query.cast_ray(
            ray3_origin,
            Dir3::NEG_Y,
            half_height * 2.0,
            true,
            &filter,
        );
        let Some(ledge_hit) = ray3_hit else {
            continue;
        };

        // Validate: surface normal is mostly upward
        if ledge_hit.normal.dot(Vec3::Y) < 0.7 {
            continue;
        }

        let surface_y = ray3_origin.y - ledge_hit.distance;

        // Validate: ledge height is between player center and above head
        let min_y = center.y;
        let max_y = center.y + half_height + 0.5;
        if surface_y < min_y || surface_y > max_y {
            continue;
        }

        jump_pressed.0 = false;
        commands.entity(entity).insert(LedgeGrabbing {
            surface_point: Vec3::new(wall_point.x, surface_y, wall_point.z),
            wall_normal: wall_hit.normal,
        });

        // Camera bounce on grab
        if let Ok(pitch_entity) = pitch_query.single() {
            commands.entity(pitch_entity).insert(LedgeGrabBounce {
                elapsed: 0.0,
                duration: 0.4,
            });
        }
    }
}

/// Applies ledge grab behavior:
/// - Hold: zeros velocity, snaps position against wall at grab height
/// - Jump (facing wall): begin animated climb
/// - Jump (looking away): wall jump off wall
/// - Crouch / backward / strafe while not facing wall: drop
/// - Strafe while facing wall: shuffle sideways along ledge
pub fn apply_ledge_grab(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut query: Query<(
        Entity,
        &mut Transform,
        &PlayerConfig,
        &mut PlayerVelocity,
        &mut LedgeGrabbing,
        &mut JumpPressed,
        &CrouchInput,
        &MoveInput,
        &mut LedgeCooldown,
    )>,
    pitch_query: Query<(Entity, Option<&LedgeShuffleBob>), With<CameraPitch>>,
    yaw_query: Query<&Transform, (With<CameraYaw>, Without<LedgeGrabbing>)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let yaw_transform = yaw_query.single().ok();
    let look_forward = yaw_transform
        .map(|t| Vec3::new(t.forward().x, 0.0, t.forward().z).normalize_or_zero());

    for (entity, mut transform, config, mut velocity, mut ledge, mut jump_pressed, crouch_input, move_input, mut cooldown) in
        &mut query
    {
        let half_height = config.stand_height / 2.0;
        let wall_normal_h = Vec3::new(ledge.wall_normal.x, 0.0, ledge.wall_normal.z).normalize_or_zero();
        let wall_into = -wall_normal_h;
        let facing_wall = look_forward
            .map(|fwd| fwd.dot(wall_into) > 0.25)
            .unwrap_or(true);

        // Helper: drop from ledge and clean up shuffle bob
        macro_rules! drop_ledge {
            () => {{
                commands.entity(entity).remove::<LedgeGrabbing>();
                cooldown.timer = 0.0;
                if let Ok((pitch_entity, _)) = pitch_query.single() {
                    commands.entity(pitch_entity).remove::<LedgeShuffleBob>();
                }
                continue;
            }};
        }

        // Walking backward (away from wall) → drop
        if move_input.y < -0.5 {
            if let Some(fwd) = look_forward {
                let right = Vec3::new(-fwd.z, 0.0, fwd.x);
                let move_dir = (fwd * move_input.y + right * move_input.x).normalize_or_zero();
                if move_dir.dot(wall_normal_h) > 0.25 {
                    drop_ledge!();
                }
            }
        }

        // Strafing while not facing wall → drop
        if move_input.x.abs() > 0.5 && !facing_wall {
            drop_ledge!();
        }

        // Jump
        if jump_pressed.0 {
            jump_pressed.0 = false;

            if let Ok((pitch_entity, _)) = pitch_query.single() {
                commands.entity(pitch_entity).remove::<LedgeShuffleBob>();
            }

            if facing_wall {
                // Climb: begin animated ledge climb
                let start_pos = transform.translation;
                let end_pos = Vec3::new(
                    ledge.surface_point.x + wall_into.x * (config.radius + 0.1),
                    ledge.surface_point.y + half_height,
                    ledge.surface_point.z + wall_into.z * (config.radius + 0.1),
                );

                velocity.0 = Vec3::ZERO;

                commands.entity(entity).insert(LedgeClimbing {
                    start_pos,
                    end_pos,
                    wall_normal: ledge.wall_normal,
                    elapsed: 0.0,
                    duration: config.ledge_climb_duration,
                });

                if let Ok((pitch_entity, _)) = pitch_query.single() {
                    let roll_sign = if rand::thread_rng().gen_bool(0.5) { 1.0 } else { -1.0 };
                    commands.entity(pitch_entity).insert(LedgeClimbBob {
                        elapsed: 0.0,
                        duration: config.ledge_climb_duration,
                        roll_sign,
                    });
                }
            } else {
                // Wall jump: launch away from wall
                velocity.0 = wall_normal_h * config.jump_velocity * 0.6 + Vec3::Y * config.jump_velocity;
                commands.entity(entity).remove::<LedgeGrabbing>();
                cooldown.timer = 0.0;
            }

            continue;
        }

        // Crouch → drop
        if crouch_input.0 {
            drop_ledge!();
        }

        // Strafing while facing wall → shuffle along ledge
        if move_input.x.abs() > 0.1 && facing_wall {
            if let Some(fwd) = look_forward {
                let wall_tangent = wall_normal_h.cross(Vec3::Y).normalize_or_zero();
                let cam_right = Vec3::new(-fwd.z, 0.0, fwd.x);
                let tangent_dot = (cam_right * move_input.x).dot(wall_tangent);

                if tangent_dot.abs() > 0.01 {
                    let shuffle_dir = wall_tangent * tangent_dot.signum();
                    let shuffle_delta = shuffle_dir * config.ledge_shuffle_speed * dt;

                    // Verify ledge still exists at the new position
                    let new_point = ledge.surface_point + shuffle_delta;
                    let ray_origin = Vec3::new(new_point.x, ledge.surface_point.y + 0.3, new_point.z);
                    let filter = SpatialQueryFilter::default().with_mask(GameLayer::World);
                    let ray_hit = spatial_query.cast_ray(
                        ray_origin,
                        Dir3::NEG_Y,
                        half_height,
                        true,
                        &filter,
                    );

                    let valid = ray_hit
                        .filter(|hit| hit.normal.dot(Vec3::Y) > 0.7);

                    if let Some(hit) = valid {
                        let new_y = ray_origin.y - hit.distance;
                        ledge.surface_point = Vec3::new(new_point.x, new_y, new_point.z);

                        // Advance shuffle bob
                        if let Ok((pitch_entity, shuffle_bob)) = pitch_query.single() {
                            let current_timer = shuffle_bob.map(|b| b.timer).unwrap_or(0.0);
                            commands.entity(pitch_entity).insert(LedgeShuffleBob {
                                timer: current_timer + dt,
                                amplitude: config.ledge_shuffle_bob_amplitude,
                            });
                        }
                    } else {
                        // No valid ledge surface — drop off the edge
                        drop_ledge!();
                    }
                }
            }
        } else {
            // Not shuffling — remove bob if present
            if let Ok((pitch_entity, Some(_))) = pitch_query.single() {
                commands.entity(pitch_entity).remove::<LedgeShuffleBob>();
            }
        }

        // Hold: zero velocity and snap position
        velocity.0 = Vec3::ZERO;

        let target_y = ledge.surface_point.y - half_height;
        transform.translation.y = target_y;

        let wall_contact = Vec3::new(ledge.surface_point.x, transform.translation.y, ledge.surface_point.z);
        let snapped = wall_contact + wall_normal_h * config.radius;
        transform.translation.x = snapped.x;
        transform.translation.z = snapped.z;
    }
}

/// Animates the two-phase ledge climb: up then forward, using smoothstep interpolation.
pub fn animate_ledge_climb(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Transform,
        &mut PlayerVelocity,
        &mut LedgeClimbing,
        &mut LedgeCooldown,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, mut velocity, mut climb, mut cooldown) in &mut query {
        climb.elapsed += dt;
        let t = (climb.elapsed / climb.duration).clamp(0.0, 1.0);

        // cubic ease-in-out
        let ease = |x: f32| {
            if x < 0.5 {
                4.0 * x * x * x
            } else {
                1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
            }
        };

        if t <= 0.5 {
            // Phase 1: move upward (t 0→0.5 maps to 0→1)
            let phase = ease(t * 2.0);
            transform.translation.y = climb.start_pos.y + (climb.end_pos.y - climb.start_pos.y) * phase;
            // XZ stays at start
            transform.translation.x = climb.start_pos.x;
            transform.translation.z = climb.start_pos.z;
        } else {
            // Phase 2: move forward (t 0.5→1.0 maps to 0→1)
            let phase = ease((t - 0.5) * 2.0);
            // Y is already at end height
            transform.translation.y = climb.end_pos.y;
            transform.translation.x = climb.start_pos.x + (climb.end_pos.x - climb.start_pos.x) * phase;
            transform.translation.z = climb.start_pos.z + (climb.end_pos.z - climb.start_pos.z) * phase;
        }

        // Keep velocity zeroed during animation
        velocity.0 = Vec3::ZERO;

        // Finished
        if t >= 1.0 {
            commands.entity(entity).remove::<LedgeClimbing>();
            commands.entity(entity).remove::<LedgeGrabbing>();
            commands.entity(entity).remove::<Crouching>();
            cooldown.timer = 0.0;
        }
    }
}
