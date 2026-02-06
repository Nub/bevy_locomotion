use bevy::prelude::*;

use super::input::{JumpHeld, JumpPressed};
use super::state::*;

/// Tracks last slide direction and time for slide-jump boost
pub fn update_last_slide(
    mut query: Query<(&mut LastSlide, Option<&Sliding>)>,
    time: Res<Time>,
) {
    for (mut last_slide, sliding) in &mut query {
        if let Some(sliding) = sliding {
            last_slide.direction = sliding.direction;
            last_slide.timer = 0.0;
        } else {
            last_slide.timer += time.delta_secs();
        }
    }
}

/// Handles jump input with coyote time and jump buffering
pub fn handle_jump(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &PlayerConfig,
        &mut PlayerVelocity,
        &mut JumpBuffer,
        &mut CoyoteTime,
        &mut JumpPressed,
        &mut LastSlide,
        Option<&Grounded>,
        Option<&Sliding>,
    )>,
    time: Res<Time>,
) {
    for (entity, config, mut velocity, mut buffer, mut coyote, mut jump_pressed, mut last_slide, grounded, sliding) in
        &mut query
    {
        // Reset vertical velocity when grounded (so gravity doesn't accumulate)
        if grounded.is_some() && velocity.y < 0.0 {
            velocity.y = 0.0;
        }

        // Update jump buffer
        if jump_pressed.0 {
            buffer.buffered = true;
            buffer.timer = 0.0;
            jump_pressed.0 = false;
        } else {
            buffer.timer += time.delta_secs();
            if buffer.timer > config.jump_buffer {
                buffer.buffered = false;
            }
        }

        // Can jump if grounded OR within coyote time, AND jump is buffered
        let can_jump =
            (grounded.is_some() || coyote.timer < config.coyote_time) && buffer.buffered;

        if can_jump {
            velocity.y = config.jump_velocity;
            buffer.buffered = false;
            coyote.timer = config.coyote_time;

            // Slide-jump boost: apply forward momentum if recently slid (once per slide)
            if (sliding.is_some() || last_slide.timer < config.slide_jump_grace)
                && last_slide.direction != Vec3::ZERO
            {
                velocity.x += last_slide.direction.x * config.slide_jump_boost;
                velocity.z += last_slide.direction.z * config.slide_jump_boost;
                last_slide.direction = Vec3::ZERO; // consume the boost
            }

            commands.entity(entity).remove::<Grounded>();
            commands.entity(entity).remove::<JumpCut>();
            commands.entity(entity).remove::<Sliding>();
            commands.entity(entity).remove::<Crouching>();
        }
    }
}

/// Implements variable jump height - releasing jump early reduces upward velocity (once per jump)
pub fn variable_jump_height(
    mut commands: Commands,
    mut query: Query<
        (Entity, &JumpHeld, &PlayerConfig, &mut PlayerVelocity),
        (Without<Grounded>, Without<JumpCut>),
    >,
) {
    for (entity, jump_held, config, mut velocity) in &mut query {
        if !jump_held.0 && velocity.y > 0.0 {
            velocity.y *= config.jump_cut_multiplier;
            commands.entity(entity).insert(JumpCut);
        }
    }
}
