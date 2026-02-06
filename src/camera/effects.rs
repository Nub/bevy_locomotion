use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

use crate::player::{Crouching, Grounded, Player, PlayerConfig, PlayerVelocity};

use super::CameraPitch;

/// Damped vertical bounce on ledge grab to sell impact weight
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct LedgeGrabBounce {
    pub elapsed: f32,
    pub duration: f32,
}

/// Head bob while shuffling sideways on a ledge
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct LedgeShuffleBob {
    pub timer: f32,
    pub amplitude: f32,
}

/// Camera pitch bob during ledge climb animation
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct LedgeClimbBob {
    pub elapsed: f32,
    pub duration: f32,
    /// -1.0 or 1.0 — which direction to roll during the climb
    pub roll_sign: f32,
}

/// FPS camera marker with effect settings
#[derive(Component)]
pub struct FpsCamera {
    /// Base FOV in radians
    pub base_fov: f32,
    /// Sprint FOV in radians
    pub sprint_fov: f32,
    /// Current FOV
    pub current_fov: f32,
    /// FOV transition speed
    pub fov_speed: f32,
    /// View punch amount (for landing effects)
    pub view_punch: f32,
    /// View punch decay rate (scales with impact)
    pub punch_decay_rate: f32,
    /// Head bob vertical amplitude in meters (0.0 to disable)
    pub head_bob_amplitude: f32,
    /// Head bob cycles per second (scaled by movement speed)
    pub head_bob_frequency: f32,
    /// Head bob lateral sway amplitude in meters
    pub head_bob_sway: f32,
    /// Internal head bob phase timer
    pub head_bob_timer: f32,
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self {
            base_fov: 90.0_f32.to_radians(),
            sprint_fov: 100.0_f32.to_radians(),
            current_fov: 90.0_f32.to_radians(),
            fov_speed: 8.0,
            view_punch: 0.0,
            punch_decay_rate: 1.0,
            head_bob_amplitude: 0.02,
            head_bob_frequency: 12.0,
            head_bob_sway: 0.01,
            head_bob_timer: 0.0,
        }
    }
}

/// Updates camera FOV based on player speed
pub fn update_fov(
    player_query: Query<(&PlayerVelocity, &PlayerConfig), With<Player>>,
    mut camera_query: Query<(&mut Projection, &mut FpsCamera)>,
    time: Res<Time>,
) {
    let Ok((velocity, config)) = player_query.single() else {
        return;
    };

    let horizontal_speed = Vec2::new(velocity.x, velocity.z).length();

    for (mut projection, mut camera) in &mut camera_query {
        // Interpolate FOV between base and sprint based on speed
        let t = ((horizontal_speed - config.walk_speed)
            / (config.sprint_speed - config.walk_speed))
            .clamp(0.0, 1.0);
        let target_fov = camera.base_fov + (camera.sprint_fov - camera.base_fov) * t;

        let dt = time.delta_secs();
        camera.current_fov += (target_fov - camera.current_fov) * camera.fov_speed * dt;

        if let Projection::Perspective(ref mut persp) = *projection {
            persp.fov = camera.current_fov;
        }
    }
}

/// Applies head bob based on movement speed
pub fn apply_head_bob(
    player_query: Query<(&PlayerVelocity, Has<Grounded>), With<Player>>,
    mut camera_query: Query<(&mut Transform, &mut FpsCamera), With<FpsCamera>>,
    time: Res<Time>,
) {
    let Ok((velocity, grounded)) = player_query.single() else {
        return;
    };

    let dt = time.delta_secs();
    let horizontal_speed = Vec3::new(velocity.x, 0.0, velocity.z).length();

    for (mut transform, mut camera) in &mut camera_query {
        if camera.head_bob_amplitude == 0.0 {
            return;
        }

        let (target_y, target_x) = if grounded && horizontal_speed > 0.5 {
            camera.head_bob_timer += dt * camera.head_bob_frequency;
            // Wrap to avoid precision loss over long sessions
            if camera.head_bob_timer > std::f32::consts::TAU * 2.0 {
                camera.head_bob_timer -= std::f32::consts::TAU * 2.0;
            }

            let t = camera.head_bob_timer;
            (
                t.sin() * camera.head_bob_amplitude,
                (t * 0.5).sin() * camera.head_bob_sway,
            )
        } else {
            (0.0, 0.0)
        };

        let lerp_speed = 10.0 * dt;
        transform.translation.y += (target_y - transform.translation.y) * lerp_speed;
        transform.translation.x += (target_x - transform.translation.x) * lerp_speed;
    }
}

/// Tracks previous state for landing detection
#[derive(Resource, Default)]
pub struct PreviousGroundedState {
    pub was_grounded: bool,
    pub last_vertical_velocity: f32,
}

/// Applies view punch on landing - scales with impact velocity
pub fn apply_view_punch(
    player_query: Query<(&LinearVelocity, Has<Grounded>), With<Player>>,
    mut camera_query: Query<&mut FpsCamera>,
    mut prev_state: ResMut<PreviousGroundedState>,
    time: Res<Time>,
) {
    let Ok((lin_vel, grounded)) = player_query.single() else {
        return;
    };

    let dt = time.delta_secs();

    for mut camera in &mut camera_query {
        // Detect landing - was airborne, now grounded
        if grounded && !prev_state.was_grounded {
            // Impact velocity (how fast we were falling)
            let impact_speed = (-prev_state.last_vertical_velocity).max(0.0);

            // Thresholds: normal jump ~4-8 m/s, big falls ~15+ m/s
            let min_impact = 2.0;  // Very small threshold - most landings have effect
            let max_impact = 18.0; // Cap for maximum effect

            if impact_speed > min_impact {
                let normalized = ((impact_speed - min_impact) / (max_impact - min_impact)).clamp(0.0, 1.0);

                // Punch magnitude: 0.015 to 0.1 radians
                camera.view_punch = 0.015 + normalized * 0.085;

                // Decay rate: much slower for longer window
                // Normal jump: ~0.4s recovery, big fall: ~1.5s recovery
                camera.punch_decay_rate = 2.5 - normalized * 1.8; // 2.5 for small, 0.7 for big
            }
        }

        // Decay view punch smoothly - exponential decay for natural feel
        if camera.view_punch > 0.0005 {
            camera.view_punch *= 1.0 - (camera.punch_decay_rate * dt);
        } else {
            camera.view_punch = 0.0;
        }
    }

    prev_state.was_grounded = grounded;
    prev_state.last_vertical_velocity = lin_vel.y;
}

/// Adjusts camera height for crouch
pub fn update_camera_height(
    player_query: Query<(&PlayerConfig, Has<Crouching>), With<Player>>,
    mut pitch_query: Query<&mut Transform, With<CameraPitch>>,
    time: Res<Time>,
) {
    let Ok((config, crouching)) = player_query.single() else {
        return;
    };

    let target_height = if crouching {
        config.crouch_height / 2.0 - 0.1
    } else {
        config.stand_height / 2.0 - 0.1
    };

    for mut transform in &mut pitch_query {
        // Smooth transition
        transform.translation.y +=
            (target_height - transform.translation.y) * 10.0 * time.delta_secs();
    }
}

/// Applies a damped vertical bounce to the camera on ledge grab.
/// Runs after `update_camera_height` so the offset layers on top.
pub fn apply_ledge_grab_bounce(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut LedgeGrabBounce), With<CameraPitch>>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut bounce) in &mut query {
        bounce.elapsed += time.delta_secs();
        if bounce.elapsed >= bounce.duration {
            commands.entity(entity).remove::<LedgeGrabBounce>();
            continue;
        }
        let t = bounce.elapsed / bounce.duration;
        // Damped sine: quick dip down, small overshoot up, settle
        let offset = (-6.0 * t).exp() * (t * std::f32::consts::TAU * 1.5).sin() * -0.07;
        transform.translation.y += offset;
    }
}

/// Applies vertical bob while shuffling on a ledge
pub fn apply_ledge_shuffle_bob(
    mut query: Query<(&mut Transform, &LedgeShuffleBob), With<CameraPitch>>,
) {
    for (mut transform, bob) in &mut query {
        let offset = (bob.timer * 10.0).sin() * bob.amplitude;
        transform.translation.y += offset;
    }
}

/// Advances the ledge climb bob timer and removes the component when done
pub fn apply_ledge_climb_bob(
    mut commands: Commands,
    mut query: Query<(Entity, &mut LedgeClimbBob), With<CameraPitch>>,
    time: Res<Time>,
) {
    for (entity, mut bob) in &mut query {
        bob.elapsed += time.delta_secs();
        if bob.elapsed >= bob.duration {
            commands.entity(entity).remove::<LedgeClimbBob>();
        }
    }
}
