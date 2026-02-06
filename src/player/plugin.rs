use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::crouch::*;
use super::input::{
    clear_look_input, handle_crouch_end, handle_crouch_start, handle_jump_end, handle_jump_start,
    handle_look_input, handle_move_end, handle_move_input, handle_sprint_end, handle_sprint_start,
    CrouchAction, CrouchInput, JumpAction, JumpHeld, JumpPressed, LookAction, LookInput,
    MoveAction, MoveInput, SprintAction, SprintInput,
};
use super::jump::*;
use super::movement::*;
use super::state::*;
use crate::camera::{CameraConfig, CameraPitch, CameraYaw, FpsCamera, PitchAngle};
use crate::physics::GameLayer;

/// Plugin for first-person player controller
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin);

        // Register input context for player
        app.add_input_context::<Player>();

        // Input observers
        app.add_observer(handle_move_input);
        app.add_observer(handle_move_end);
        app.add_observer(handle_look_input);
        app.add_observer(handle_sprint_start);
        app.add_observer(handle_sprint_end);
        app.add_observer(handle_crouch_start);
        app.add_observer(handle_crouch_end);
        app.add_observer(handle_jump_start);
        app.add_observer(handle_jump_end);

        // Spawn player on startup
        app.add_systems(Startup, spawn_player);

        // Fixed update systems for physics
        app.add_systems(
            FixedUpdate,
            (
                update_grounded_state,
                update_sprint_state,
                update_crouch_state,
                update_last_slide,
                handle_jump,
                variable_jump_height,
                ground_movement,
                air_movement,
                apply_slide,
                apply_gravity,
                apply_velocity,
                update_collider_height,
            )
                .chain(),
        );

        // Clear look input at end of frame (jump is cleared in FixedUpdate)
        app.add_systems(Last, clear_look_input);
    }
}

/// Spawns the player entity with all required components
fn spawn_player(mut commands: Commands) {
    let config = PlayerConfig::default();

    // Spawn yaw entity (rotates on Y axis for left/right look)
    let yaw_entity = commands
        .spawn((
            CameraYaw,
            Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
            Visibility::default(),
        ))
        .id();

    // Spawn pitch entity as child (rotates on X axis for up/down look)
    let pitch_entity = commands
        .spawn((
            CameraPitch,
            PitchAngle::default(),
            CameraConfig::default(),
            Transform::from_translation(Vec3::new(0.0, config.stand_height / 2.0 - 0.1, 0.0)),
            Visibility::default(),
        ))
        .id();

    // Spawn camera as child of pitch
    let camera_entity = commands
        .spawn((
            FpsCamera::default(),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 90.0_f32.to_radians(),
                ..default()
            }),
            Transform::default(),
        ))
        .id();

    // Set up hierarchy: yaw -> pitch -> camera
    commands.entity(yaw_entity).add_child(pitch_entity);
    commands.entity(pitch_entity).add_child(camera_entity);

    // Spawn player body
    let capsule_height = config.stand_height - config.radius * 2.0;

    commands
        .spawn((
            Player,
            config,
            PlayerVelocity::default(),
            CoyoteTime::default(),
            JumpBuffer::default(),
            AirTime::default(),
            SprintGrace::default(),
            LastSlide::default(),
        ))
        .insert((
            // Input state
            MoveInput::default(),
            LookInput::default(),
            SprintInput::default(),
            CrouchInput::default(),
            JumpPressed::default(),
            JumpHeld::default(),
        ))
        .insert((
            // Physics - Dynamic body with locked rotation, let Avian handle collisions
            RigidBody::Dynamic,
            Collider::capsule(config.radius, capsule_height),
            CollisionLayers::new(GameLayer::Player, [GameLayer::World, GameLayer::Trigger]),
            LockedAxes::ROTATION_LOCKED,
            LinearVelocity::default(),
            TranslationInterpolation,
            Friction::new(0.0),  // No friction - we handle movement ourselves
            Restitution::new(0.0),  // No bounce
            GravityScale(0.0),  // We handle gravity ourselves for more control
        ))
        .insert((
            // Transform
            Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
            Visibility::default(),
        ))
        .insert(
            // Input bindings
            actions!(Player[
                (
                    Action::<MoveAction>::new(),
                    bindings![
                        (KeyCode::KeyW, SwizzleAxis::YXZ),
                        (KeyCode::KeyS, SwizzleAxis::YXZ, Negate::all()),
                        KeyCode::KeyD,
                        (KeyCode::KeyA, Negate::all()),
                    ],
                ),
                (
                    Action::<LookAction>::new(),
                    bindings![
                        Binding::mouse_motion(),
                    ],
                ),
                (
                    Action::<JumpAction>::new(),
                    bindings![KeyCode::Space, GamepadButton::South],
                ),
                (
                    Action::<SprintAction>::new(),
                    bindings![KeyCode::ShiftLeft, GamepadButton::LeftTrigger],
                ),
                (
                    Action::<CrouchAction>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::RightThumb],
                ),
            ]),
        );
}
