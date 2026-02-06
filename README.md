# bevy_locomotion

A first-person character controller for Bevy 0.18 and Avian3d. All
movement is driven by physics raycasts via Avian3d spatial queries.

- **Walk and sprint** with acceleration/friction ground movement model
- **Jump** with variable height (release early for short hops), coyote time, and jump buffering
- **Crouch** with collider resizing and stand-up obstruction checks
- **Slide** by sprinting into crouch — momentum-based with a friction curve and configurable boost
- **Slide jump** for a forward momentum boost when jumping out of a slide
- **Ledge grab** by pressing jump near a wall edge while airborne (requires `LedgeGrabbable` marker)
- **Ledge climb** with a two-phase animated mantle (up then forward)
- **Ledge shuffle** by strafing while hanging, with head bob
- **Wall jump** by looking away from the wall and jumping while grabbing a ledge
- **Ladder climbing** on surfaces marked with `Ladder` — press up to grab, jump to dismount
- **Forced slide** on surfaces marked with `ForceSlide` — player is pushed downhill by gravity
- **Auto step-up** over small obstacles like stairs and curbs
- **Slope handling** with velocity projection to maintain speed on inclines
- **Air control** with reduced acceleration while airborne
- **Audio events** emitted as messages for footsteps, jumps, landings, slides, ledge grabs, and more
- **Configurable collision layers** — bring your own `PhysicsLayer` enum or use the built-in `GameLayer`

## Quick Start

Add the dependency:

```toml
[dependencies]
bevy_locomotion = { git = "https://github.com/Nub/bevy_locomotion" }
```

Minimal example:

```rust
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_locomotion::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyLocomotionPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Player
    spawn_player(&mut commands, PlayerConfig::default(), Vec3::new(0.0, 2.0, 0.0));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
        CollisionLayers::new(GameLayer::World, [GameLayer::Player]),
    ));

    // Light
    commands.spawn((
        DirectionalLight { shadows_enabled: true, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.5, 0.0)),
    ));
}
```

`BevyLocomotionPlugin` bundles physics (Avian3d), player systems, and
camera management. Call `spawn_player` to create the player entity with all
required components, a camera hierarchy, and default WASD+mouse input
bindings.

## Controls

| Action  | Key                        |
|---------|----------------------------|
| Move    | W / A / S / D              |
| Look    | Mouse                      |
| Jump    | Space                      |
| Sprint  | Left Shift                 |
| Crouch  | Left Ctrl                  |

Sprint + Crouch initiates a **slide**. Jump during a slide for a momentum
boost. While airborne, press Jump near a wall to **ledge grab**, then Jump
again to climb or look away and Jump to wall-jump. Small obstacles are
**auto-stepped** when walking into them.

## Configuration

All movement parameters live in `PlayerConfig`. Override any field:

```rust
spawn_player(
    &mut commands,
    PlayerConfig {
        walk_speed: 6.0,
        sprint_speed: 10.0,
        jump_velocity: 9.0,
        step_up_height: 0.4,
        ..default()
    },
    Vec3::new(0.0, 2.0, 0.0),
);
```

| Field | Default | Description |
|---|---|---|
| `walk_speed` | `5.0` | Walking speed (m/s) |
| `sprint_speed` | `8.0` | Sprinting speed (m/s) |
| `crouch_speed` | `2.5` | Crouching speed (m/s) |
| `ground_accel` | `50.0` | Ground acceleration |
| `ground_friction` | `40.0` | Ground deceleration |
| `air_accel` | `15.0` | Air control acceleration |
| `jump_velocity` | `8.0` | Jump impulse (m/s) |
| `jump_cut_multiplier` | `0.5` | Variable jump height cut (0.0-1.0) |
| `coyote_time` | `0.15` | Coyote time window (s) |
| `jump_buffer` | `0.1` | Jump buffer window (s) |
| `stand_height` | `1.8` | Standing capsule height (m) |
| `crouch_height` | `1.0` | Crouching capsule height (m) |
| `radius` | `0.4` | Capsule radius (m) |
| `min_slide_speed` | `6.0` | Minimum speed to start a slide (m/s) |
| `slide_duration` | `0.8` | Slide duration (s) |
| `slide_friction` | `2.0` | Slide friction curve exponent |
| `slide_boost` | `1.2` | Slide initiation speed multiplier |
| `sprint_slide_grace` | `0.15` | Grace period after releasing sprint for slides (s) |
| `slide_jump_boost` | `3.0` | Forward boost when jumping out of a slide (m/s) |
| `slide_jump_grace` | `0.2` | Grace period after slide for slide-jump boost (s) |
| `max_horizontal_speed` | `20.0` | Speed cap (m/s), 0 = uncapped |
| `ledge_detect_reach` | `0.6` | Ledge probe distance past capsule (m) |
| `ledge_climb_duration` | `1.05` | Climb animation duration (s) |
| `ledge_shuffle_speed` | `1.75` | Sideways shuffle speed on ledge (m/s) |
| `ledge_cooldown` | `0.4` | Cooldown before re-grabbing a ledge (s) |
| `ledge_grab_max_fall_speed` | `10.0` | Max fall speed for ledge grab (m/s), 0 = uncapped |
| `ledge_grab_ascending` | `false` | Allow ledge grab while moving upward |
| `ladder_climb_speed` | `4.0` | Ladder climbing speed (m/s) |
| `max_slope_angle` | `39.0` | Maximum walkable slope angle (degrees) |
| `step_up_height` | `0.35` | Max auto-step obstacle height (m) |
| `player_layer` | `GameLayer::Player` | Physics layer for the player body |
| `world_layer` | `GameLayer::World` | Layer mask for spatial queries (ground, ledge, step-up, crouch) |
| `collision_mask` | `World + Trigger` | Layer mask the player rigid body collides with |

## Custom Collision Layers

By default the controller uses `GameLayer` for physics queries. To use your
own layer enum, set `player_layer`, `world_layer`, and `collision_mask` on
`PlayerConfig`:

```rust
use avian3d::prelude::*;

#[derive(PhysicsLayer, Default)]
enum MyLayer {
    #[default]
    Default,
    Player,
    Environment,
    Trigger,
}

spawn_player(
    &mut commands,
    PlayerConfig {
        player_layer: MyLayer::Player.into(),
        world_layer: MyLayer::Environment.into(),
        collision_mask: LayerMask::from([MyLayer::Environment, MyLayer::Trigger]),
        ..default()
    },
    Vec3::new(0.0, 2.0, 0.0),
);
```

World geometry should collide with the player layer you choose:

```rust
CollisionLayers::new(MyLayer::Environment, [MyLayer::Player])
```

Add `LedgeGrabbable` to walls that should support ledge grabs, `Ladder` to
climbable surfaces (use `Sensor` on the trigger layer), and `ForceSlide` to
ramps that force the player downhill.

## Querying Player State

The player's current state is expressed as marker components. Query them in
your own systems:

```rust
fn my_system(
    query: Query<(
        &PlayerVelocity,
        &Transform,
        Has<Grounded>,
        Has<Sprinting>,
        Has<Crouching>,
        Has<Sliding>,
        Has<LedgeGrabbing>,
        Has<LedgeClimbing>,
        Has<OnLadder>,
        Has<ForcedSliding>,
    ), With<Player>>,
) {
    let Ok((velocity, transform, grounded, ..)) = query.single() else { return };
    // ...
}
```

## Audio Events

The controller emits `PlayerAudioMessage` messages for gameplay events.
Subscribe with a `MessageReader` to play sounds, spawn particles, or
trigger any other feedback:

```rust
fn play_sounds(mut reader: MessageReader<PlayerAudioMessage>) {
    for msg in reader.read() {
        match msg {
            PlayerAudioMessage::Footstep { speed } => { /* play footstep */ }
            PlayerAudioMessage::Landed { impact_speed } => { /* thud */ }
            PlayerAudioMessage::Jumped => { /* whoosh */ }
            PlayerAudioMessage::SlideStart => { /* screech */ }
            PlayerAudioMessage::SlideEnd => { /* fade */ }
            PlayerAudioMessage::LedgeGrabbed => { /* clunk */ }
            PlayerAudioMessage::LedgeClimbStarted => { /* effort */ }
            PlayerAudioMessage::LedgeClimbFinished => { /* done */ }
            PlayerAudioMessage::WallJumped => { /* kick */ }
            PlayerAudioMessage::SteppedUp => { /* tap */ }
            PlayerAudioMessage::LadderEnter => { /* grab */ }
            PlayerAudioMessage::LadderExit => { /* release */ }
            PlayerAudioMessage::ForcedSlideStart => { /* whoosh */ }
            PlayerAudioMessage::ForcedSlideEnd => { /* stop */ }
        }
    }
}
```

## Collision Layers

World geometry must be on `GameLayer::World` to interact with the player:

```rust
CollisionLayers::new(GameLayer::World, [GameLayer::Player])
```

The player is spawned on `GameLayer::Player` and collides with `World` and
`Trigger` layers.

## Gymnasium Example

A test environment with slopes, jump gaps, obstacles, crouch tunnels, ledge
walls, and slide ramps:

```sh
cargo run --example gymnasium
```

Enable placeholder audio with `--features gym-audio`.
