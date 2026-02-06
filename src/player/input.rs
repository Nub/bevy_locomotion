use bevy::ecs::observer::On;
use bevy::prelude::{Component, Deref, DerefMut, EntityEvent, Query, Vec2};
use bevy_enhanced_input::prelude::*;

/// Move in a direction (WASD)
#[derive(Debug, InputAction)]
#[action_output(Vec2)]
pub struct MoveAction;

/// Look around (mouse delta)
#[derive(Debug, InputAction)]
#[action_output(Vec2)]
pub struct LookAction;

/// Jump action
#[derive(Debug, InputAction)]
#[action_output(bool)]
pub struct JumpAction;

/// Sprint action (hold)
#[derive(Debug, InputAction)]
#[action_output(bool)]
pub struct SprintAction;

/// Crouch action
#[derive(Debug, InputAction)]
#[action_output(bool)]
pub struct CrouchAction;

/// Stores the current movement input vector
#[derive(Component, Default, Deref, DerefMut)]
pub struct MoveInput(pub Vec2);

/// Stores the current look input delta
#[derive(Component, Default, Deref, DerefMut)]
pub struct LookInput(pub Vec2);

/// Stores whether sprint is held
#[derive(Component, Default, Deref, DerefMut)]
pub struct SprintInput(pub bool);

/// Stores whether crouch is held
#[derive(Component, Default, Deref, DerefMut)]
pub struct CrouchInput(pub bool);

/// Stores whether jump was pressed this frame
#[derive(Component, Default)]
pub struct JumpPressed(pub bool);

/// Stores whether jump is currently held
#[derive(Component, Default, Deref, DerefMut)]
pub struct JumpHeld(pub bool);

/// System to handle move input via observer
pub fn handle_move_input(trigger: On<Fire<MoveAction>>, mut query: Query<&mut MoveInput>) {
    if let Ok(mut move_input) = query.get_mut(trigger.event_target()) {
        move_input.0 = trigger.value;
    }
}

/// Clear move input when all movement keys are released
pub fn handle_move_end(trigger: On<Complete<MoveAction>>, mut query: Query<&mut MoveInput>) {
    if let Ok(mut move_input) = query.get_mut(trigger.event_target()) {
        move_input.0 = Vec2::ZERO;
    }
}

/// System to handle look input via observer
pub fn handle_look_input(trigger: On<Fire<LookAction>>, mut query: Query<&mut LookInput>) {
    if let Ok(mut look_input) = query.get_mut(trigger.event_target()) {
        look_input.0 = trigger.value;
    }
}

/// Handle sprint start
pub fn handle_sprint_start(trigger: On<Start<SprintAction>>, mut query: Query<&mut SprintInput>) {
    if let Ok(mut sprint) = query.get_mut(trigger.event_target()) {
        sprint.0 = true;
    }
}

/// Handle sprint end
pub fn handle_sprint_end(trigger: On<Complete<SprintAction>>, mut query: Query<&mut SprintInput>) {
    if let Ok(mut sprint) = query.get_mut(trigger.event_target()) {
        sprint.0 = false;
    }
}

/// Handle crouch start
pub fn handle_crouch_start(trigger: On<Start<CrouchAction>>, mut query: Query<&mut CrouchInput>) {
    if let Ok(mut crouch) = query.get_mut(trigger.event_target()) {
        crouch.0 = true;
    }
}

/// Handle crouch end
pub fn handle_crouch_end(trigger: On<Complete<CrouchAction>>, mut query: Query<&mut CrouchInput>) {
    if let Ok(mut crouch) = query.get_mut(trigger.event_target()) {
        crouch.0 = false;
    }
}

/// Handle jump press
pub fn handle_jump_start(
    trigger: On<Start<JumpAction>>,
    mut pressed_query: Query<&mut JumpPressed>,
    mut held_query: Query<&mut JumpHeld>,
) {
    let entity = trigger.event_target();
    if let Ok(mut jump) = pressed_query.get_mut(entity) {
        jump.0 = true;
    }
    if let Ok(mut held) = held_query.get_mut(entity) {
        held.0 = true;
    }
}

/// Handle jump release
pub fn handle_jump_end(trigger: On<Complete<JumpAction>>, mut query: Query<&mut JumpHeld>) {
    if let Ok(mut held) = query.get_mut(trigger.event_target()) {
        held.0 = false;
    }
}

/// Clears jump pressed flag each frame (should run at end of frame)
pub fn clear_jump_pressed(mut query: Query<&mut JumpPressed>) {
    for mut jump in &mut query {
        jump.0 = false;
    }
}

/// Clears look input each frame
pub fn clear_look_input(mut query: Query<&mut LookInput>) {
    for mut look in &mut query {
        look.0 = Vec2::ZERO;
    }
}
