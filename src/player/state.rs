use bevy::prelude::*;

/// Marker component for the player entity (also used as input context)
#[derive(Component, Default)]
pub struct Player;

/// Player movement configuration
#[derive(Component, Clone, Copy)]
pub struct PlayerConfig {
    /// Walking speed in m/s
    pub walk_speed: f32,
    /// Sprinting speed in m/s
    pub sprint_speed: f32,
    /// Crouching speed in m/s
    pub crouch_speed: f32,
    /// Ground acceleration
    pub ground_accel: f32,
    /// Ground friction/deceleration
    pub ground_friction: f32,
    /// Air acceleration (reduced control)
    pub air_accel: f32,
    /// Jump impulse velocity
    pub jump_velocity: f32,
    /// Multiplier applied to upward velocity when jump is released early (0.0-1.0)
    pub jump_cut_multiplier: f32,
    /// Coyote time duration in seconds
    pub coyote_time: f32,
    /// Jump buffer duration in seconds
    pub jump_buffer: f32,
    /// Standing collider height
    pub stand_height: f32,
    /// Crouching collider height
    pub crouch_height: f32,
    /// Collider radius
    pub radius: f32,
    /// Minimum horizontal speed to initiate a slide (m/s)
    pub min_slide_speed: f32,
    /// Slide duration in seconds
    pub slide_duration: f32,
    /// Slide friction curve exponent (1.0 = linear, 2.0 = quadratic, higher = more speed retained early)
    pub slide_friction: f32,
    /// Slide velocity boost on initiation
    pub slide_boost: f32,
    /// Grace period after releasing sprint where slides can still initiate (seconds)
    pub sprint_slide_grace: f32,
    /// Forward momentum boost when jumping during or just after a slide (m/s)
    pub slide_jump_boost: f32,
    /// Grace period after slide ends where slide-jump boost still applies (seconds)
    pub slide_jump_grace: f32,
    /// Maximum horizontal speed (m/s), 0.0 = uncapped
    pub max_horizontal_speed: f32,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            walk_speed: 5.0,
            sprint_speed: 8.0,
            crouch_speed: 2.5,
            ground_accel: 50.0,
            ground_friction: 40.0,
            air_accel: 15.0,
            jump_velocity: 8.0,
            jump_cut_multiplier: 0.5,
            coyote_time: 0.15,
            jump_buffer: 0.1,
            stand_height: 1.8,
            crouch_height: 1.0,
            radius: 0.4,
            min_slide_speed: 6.0,
            slide_duration: 0.8,
            slide_friction: 2.0,
            slide_boost: 1.2,
            sprint_slide_grace: 0.15,
            slide_jump_boost: 3.0,
            slide_jump_grace: 0.2,
            max_horizontal_speed: 20.0,
        }
    }
}

/// Current player velocity
#[derive(Component, Default, Deref, DerefMut)]
pub struct PlayerVelocity(pub Vec3);

/// Marker: player is on the ground
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

/// Ground surface normal (set when grounded)
#[derive(Component)]
pub struct GroundNormal(pub Vec3);

/// Marker: player is sprinting
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Sprinting;

/// Marker: player is crouching
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Crouching;

/// Player is sliding
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Sliding {
    /// Direction of the slide
    pub direction: Vec3,
    /// Time when slide started
    pub start_time: f32,
    /// Initial velocity when slide started
    pub initial_speed: f32,
}

/// Tracks time since sprinting ended (for sprint-slide grace period)
#[derive(Component, Default)]
pub struct SprintGrace {
    pub timer: f32,
}

/// Marker: slide should initiate on landing (crouch pressed while airborne)
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PendingSlide;

/// Tracks the most recent slide for slide-jump boost
#[derive(Component, Default)]
pub struct LastSlide {
    pub direction: Vec3,
    pub timer: f32,
}

/// Marker: variable jump height cut has been applied this jump
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct JumpCut;

/// Coyote time tracking
#[derive(Component, Default)]
pub struct CoyoteTime {
    /// Time since leaving ground
    pub timer: f32,
}

/// Jump buffer tracking
#[derive(Component, Default)]
pub struct JumpBuffer {
    /// Time since jump was pressed
    pub timer: f32,
    /// Whether a jump is buffered
    pub buffered: bool,
}

/// Tracks the last time player was grounded (for fall damage, landing effects)
#[derive(Component, Default)]
pub struct AirTime {
    pub duration: f32,
}
