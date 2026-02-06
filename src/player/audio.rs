use bevy::prelude::*;

use super::state::*;

/// Audio event messages emitted by the player controller.
///
/// Consumers subscribe with `MessageReader<PlayerAudioMessage>` to trigger
/// sound effects, particles, or other feedback.
#[derive(Message, Clone, Debug)]
pub enum PlayerAudioMessage {
    Footstep { speed: f32 },
    Landed { impact_speed: f32 },
    Jumped,
    SlideStart,
    SlideEnd,
    LedgeGrabbed,
    LedgeClimbStarted,
    LedgeClimbFinished,
    WallJumped,
    SteppedUp,
    LadderEnter,
    LadderExit,
    ForcedSlideStart,
    ForcedSlideEnd,
}

/// Tracks previous-frame state for edge detection in audio event emission.
#[derive(Resource, Default)]
pub struct AudioTracker {
    pub was_grounded: bool,
    pub was_sliding: bool,
    pub was_ledge_grabbing: bool,
    pub was_ledge_climbing: bool,
    pub was_on_ladder: bool,
    pub was_forced_sliding: bool,
    pub last_vertical_velocity: f32,
    pub footstep_timer: f32,
}

/// Compares current player state against `AudioTracker` and emits
/// `PlayerAudioMessage` events for state transitions.
pub fn emit_player_audio_messages(
    query: Query<
        (
            &PlayerConfig,
            &PlayerVelocity,
            Has<Grounded>,
            Has<Sliding>,
            Has<LedgeGrabbing>,
            Has<LedgeClimbing>,
            Has<OnLadder>,
            Has<ForcedSliding>,
        ),
        With<Player>,
    >,
    mut tracker: ResMut<AudioTracker>,
    mut writer: MessageWriter<PlayerAudioMessage>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    let Ok((config, velocity, grounded, sliding, ledge_grabbing, ledge_climbing, on_ladder, forced_sliding)) =
        query.single()
    else {
        return;
    };

    // --- Landing ---
    if !tracker.was_grounded && grounded {
        let impact_speed = (-tracker.last_vertical_velocity).max(0.0);
        if impact_speed > 1.0 {
            writer.write(PlayerAudioMessage::Landed { impact_speed });
        }
        tracker.footstep_timer = 0.0;
    }

    // --- Jumped ---
    if tracker.was_grounded && !grounded && velocity.y > 0.0 {
        writer.write(PlayerAudioMessage::Jumped);
    }

    // --- Footsteps ---
    if grounded {
        let h_speed = Vec2::new(velocity.x, velocity.z).length();
        if h_speed > 0.5 {
            let speed_ratio = h_speed / config.walk_speed;
            let interval = 0.5 / speed_ratio;
            tracker.footstep_timer += dt;
            if tracker.footstep_timer >= interval {
                tracker.footstep_timer -= interval;
                writer.write(PlayerAudioMessage::Footstep { speed: h_speed });
            }
        } else {
            tracker.footstep_timer = 0.0;
        }
    }

    // --- Slide ---
    if !tracker.was_sliding && sliding {
        writer.write(PlayerAudioMessage::SlideStart);
    }
    if tracker.was_sliding && !sliding {
        writer.write(PlayerAudioMessage::SlideEnd);
    }

    // --- Wall jump (must check before ledge grab transition) ---
    if tracker.was_ledge_grabbing && !ledge_grabbing && !ledge_climbing && velocity.y > 0.0 {
        writer.write(PlayerAudioMessage::WallJumped);
    }

    // --- Ledge grab ---
    if !tracker.was_ledge_grabbing && ledge_grabbing {
        writer.write(PlayerAudioMessage::LedgeGrabbed);
    }

    // --- Ledge climb ---
    if !tracker.was_ledge_climbing && ledge_climbing {
        writer.write(PlayerAudioMessage::LedgeClimbStarted);
    }
    if tracker.was_ledge_climbing && !ledge_climbing {
        writer.write(PlayerAudioMessage::LedgeClimbFinished);
    }

    // --- Ladder ---
    if !tracker.was_on_ladder && on_ladder {
        writer.write(PlayerAudioMessage::LadderEnter);
    }
    if tracker.was_on_ladder && !on_ladder {
        writer.write(PlayerAudioMessage::LadderExit);
    }

    // --- Forced slide ---
    if !tracker.was_forced_sliding && forced_sliding {
        writer.write(PlayerAudioMessage::ForcedSlideStart);
    }
    if tracker.was_forced_sliding && !forced_sliding {
        writer.write(PlayerAudioMessage::ForcedSlideEnd);
    }

    // --- Update tracker ---
    tracker.was_grounded = grounded;
    tracker.was_sliding = sliding;
    tracker.was_ledge_grabbing = ledge_grabbing;
    tracker.was_ledge_climbing = ledge_climbing;
    tracker.was_on_ladder = on_ladder;
    tracker.was_forced_sliding = forced_sliding;
    tracker.last_vertical_velocity = velocity.y;
}
