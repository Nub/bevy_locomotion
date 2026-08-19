use avian3d::{math::*, prelude::*};
use bevy::prelude::*;

use super::input::MoveInput;
use super::state::*;
use crate::camera::CameraYaw;

/// Gap kept between the collider and the ground when snapping down.
/// Matches `MoveAndSlideConfig::skin_width`'s default.
const GROUND_SKIN_WIDTH: f32 = 0.01;

/// Updates grounded state via raycast
pub fn update_grounded_state(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut query: Query<(
        Entity,
        &Transform,
        &PlayerConfig,
        &LinearVelocity,
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
        // The capsule's curved bottom sits higher above slopes than flat ground.
        // Vertical distance from center to slope = (halfHeight - radius) + radius/cos(angle).
        // Using radius as the margin handles slopes up to ~60°.
        let ground_check_dist = config.stand_height / 2.0 + config.radius;

        let filter = SpatialQueryFilter::default()
            .with_mask(config.world_layer);

        let hit = spatial_query.cast_ray(
            ray_origin,
            ray_dir,
            ground_check_dist,
            true,
            &filter,
        );

        let min_ground_normal_y = config.max_slope_angle.to_radians().cos();

        let is_grounded = hit.as_ref()
            .is_some_and(|h| {
                h.distance < ground_check_dist
                    && player_vel.y < 1.0
                    && h.normal.dot(Vec3::Y) >= min_ground_normal_y
            });

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
        }
    }
}

/// Applies ground movement - sets horizontal velocity
pub fn ground_movement(
    mut query: Query<
        (
            &MoveInput,
            &PlayerConfig,
            &mut LinearVelocity,
            Has<Sprinting>,
            Has<Crouching>,
        ),
        (With<Grounded>, Without<Sliding>, Without<ForcedSliding>, Without<OnLadder>),
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
        (&MoveInput, &PlayerConfig, &mut LinearVelocity),
        (Without<Grounded>, Without<LedgeGrabbing>, Without<LedgeClimbing>, Without<OnLadder>),
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

/// Applies gravity to the player when not grounded.
///
/// **`With<Player>` is load-bearing.** The player carries `GravityScale(0.0)`
/// so that this system, and not the solver, owns its fall — but without a
/// filter the query matched *every* body in the world with a `LinearVelocity`,
/// player or not. Avian deliberately skips non-dynamic bodies when it applies
/// gravity; this system did not, so every kinematic body in the host game — the
/// ones whose motion is authored by hand, precisely because they must not fall
/// — was quietly given 20 m/s² downward anyway. In Aeonic that dragged every
/// enemy through the deck into the substrate beneath it, where the navmesh
/// could not see them and they were culled, and it drooped every projectile in
/// the game. Every other system in this chain is scoped to the player by asking
/// for `PlayerConfig` or `MoveInput`; this one had nothing to scope it.
///
/// # The shape of the fall
///
/// The pull is scaled by [`gravity_scale`] rather than applied flat, so a host
/// can ask for an arc that rises, hangs and then drops without introducing a
/// second writer of the same axis on a different schedule. See
/// [`PlayerConfig::fall_gravity_scale`]. With the neutral defaults this is
/// exactly `gravity · dt`, as it always was.
pub fn apply_gravity(
    mut query: Query<
        (&mut LinearVelocity, &PlayerConfig),
        (
            With<Player>,
            Without<Grounded>,
            Without<LedgeGrabbing>,
            Without<LedgeClimbing>,
            Without<OnLadder>,
        ),
    >,
    gravity: Res<Gravity>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut velocity, config) in &mut query {
        let scale = gravity_scale(config, velocity.0.y);
        velocity.0 += gravity.0 * scale * dt;
    }
}

/// How hard the world pulls at a given vertical speed, as a multiple of
/// `Gravity`.
///
/// Three bands, and no more than three:
///
///   * **still** — inside `gravity_shape_epsilon` of zero, nothing is shaped.
///   * **apex** — within `apex_band` of the top of the arc, `apex_gravity_scale`
///     (below 1.0 buys hang time).
///   * **descent** — past that on the way down, `fall_gravity_scale`.
///
/// A rise outside the apex band is left at 1.0 on purpose: the height a jump
/// reaches is the level designer's number, and re-scaling the climb changes
/// every gap in the level. The asymmetry belongs in the *return*.
pub fn gravity_scale(config: &PlayerConfig, vertical_speed: f32) -> f32 {
    let speed = vertical_speed.abs();
    if speed <= config.gravity_shape_epsilon {
        1.0
    } else if speed <= config.apex_band {
        config.apex_gravity_scale
    } else if vertical_speed < 0.0 {
        config.fall_gravity_scale
    } else {
        1.0
    }
}

/// Rotates `velocity` into the plane defined by `normal`, keeping its horizontal speed.
///
/// The movement systems produce a world-horizontal velocity. On a slope that vector
/// points into the surface going uphill and off it going downhill, so it has to be
/// tilted into the surface before it is used to move the character. Rescaling to the
/// original horizontal speed keeps walk speed constant regardless of incline, rather
/// than losing the component that got redirected vertically.
fn project_onto_plane(velocity: Vec3, normal: Vec3) -> Vec3 {
    let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
    let horizontal_speed = horizontal.length();

    if horizontal_speed < 0.01 {
        return velocity;
    }

    let projected = horizontal - normal * horizontal.dot(normal);
    let projected_horizontal = Vec2::new(projected.x, projected.z).length();

    if projected_horizontal < 0.001 {
        return velocity;
    }

    let mut result = projected * (horizontal_speed / projected_horizontal);
    // Preserve any residual fall speed so the character is never pushed upward.
    result.y += velocity.y.min(0.0);
    result
}

/// Casts the collider straight down and returns the position that puts the character
/// back in contact with walkable ground, if any is within `ground_snap_distance`.
///
/// Velocity projection alone cannot cover convex breaks — the lip where flat ground
/// meets a descending slope, or where a slope steepens — because the projection uses
/// the normal of the surface the character was standing on, which is still the old,
/// shallower one. Without this the character launches off every such edge.
fn snap_to_ground(
    spatial_query: &SpatialQuery,
    transform: &Transform,
    collider: &Collider,
    config: &PlayerConfig,
) -> Option<Vec3> {
    if config.ground_snap_distance <= 0.0 {
        return None;
    }

    let filter = SpatialQueryFilter::default().with_mask(config.world_layer);

    let hit = spatial_query.cast_shape(
        collider,
        transform.translation,
        transform.rotation,
        Dir3::NEG_Y,
        &ShapeCastConfig::from_max_distance(config.ground_snap_distance),
        &filter,
    )?;

    // Never snap onto walls or slopes too steep to stand on.
    let min_ground_normal_y = config.max_slope_angle.to_radians().cos();
    if hit.normal1.dot(Vec3::Y) < min_ground_normal_y {
        return None;
    }

    // Leave the same gap move-and-slide maintains, so the snap does not create a
    // penetration that depenetration has to undo on the next tick.
    let drop = hit.distance - GROUND_SKIN_WIDTH;
    if drop <= 0.0 {
        return None;
    }

    Some(transform.translation - Vec3::Y * drop)
}

/// Performs move-and-slide, following the ground surface when grounded
pub fn apply_velocity(
    mut query: Query<
        (
            Entity,
            Option<&Grounded>, Option<&GroundNormal>,
            &PlayerConfig,
            &mut Transform,
            &mut LinearVelocity,
            &Collider,
        ),
        With<Player>,
    >,
    move_and_slide: MoveAndSlide,
    spatial_query: SpatialQuery,
    time: Res<Time>,
) {
    for (
        entity,
        grounded,
        ground_normal,
        config,
        mut transform,
        mut lin_vel,
        collider
    ) in &mut query
    {
        // Clamp horizontal speed
        if config.max_horizontal_speed > 0.0 {
            let h_speed = Vec2::new(lin_vel.x, lin_vel.z).length();
            if h_speed > config.max_horizontal_speed {
                let scale = config.max_horizontal_speed / h_speed;
                lin_vel.x *= scale;
                lin_vel.z *= scale;
            }
        }

        // Only follow the ground while standing on it and not moving upward — a jump
        // must be free to leave the surface.
        let follow_ground = grounded.is_some() && lin_vel.y <= 0.0;

        let mut move_velocity = lin_vel.0;
        if follow_ground && let Some(GroundNormal(normal)) = ground_normal {
            move_velocity = project_onto_plane(move_velocity, *normal);
        }

        let MoveAndSlideOutput {
            position: new_position,
            ..
        } = move_and_slide.move_and_slide(
            collider,
            transform.translation.adjust_precision(),
            transform.rotation.adjust_precision(),
            move_velocity,
            time.delta(),
            &MoveAndSlideConfig::default(),
            &SpatialQueryFilter::from_excluded_entities([entity]),
            |hit| {
                // While grounded, slide along whatever we hit at full speed instead of
                // bleeding it off into the contact normal.
                if grounded.is_some() {
                    let normal = hit.normal.adjust_precision();
                    *hit.velocity = project_onto_plane(*hit.velocity, normal);
                }

                // Accept the hit and continue the move-and-slide algorithm with the modified velocity.
                MoveAndSlideHitResponse::Accept
            },
        );

        // Update position to the final position calculated by move-and-slide.
        transform.translation = new_position.f32();

        if follow_ground
            && let Some(snapped) = snap_to_ground(&spatial_query, &transform, collider, config)
        {
            transform.translation = snapped;
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
