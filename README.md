# player_controller

A first-person character controller for [Bevy](https://bevyengine.org/) with physics powered by [Avian3D](https://github.com/Jondolf/avian).

## Features

- Ground movement with acceleration/friction model
- Sprinting, crouching, and sliding
- Variable-height jumping with coyote time and jump buffering
- FPS camera with head bob, FOV effects, and view punch on landing
- Cursor grab/release (Escape to release, click to recapture)

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
player_controller = { path = "../player_controller" }
bevy = "0.18"
```

Then in your app:

```rust
use bevy::prelude::*;
use player_controller::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerControllerPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn player at a position
    spawn_player(&mut commands, PlayerConfig::default(), Vec3::new(0.0, 2.0, 0.0));

    // Add your world geometry, lighting, etc.
}
```

`PlayerControllerPlugin` bundles physics (Avian3D), player input/movement, and camera systems. You can also add the sub-plugins individually:

```rust
app.add_plugins((PhysicsPlugin, PlayerPlugin, CameraPlugin));
```

## Configuring the Player

`PlayerConfig` exposes all movement tuning parameters:

```rust
let config = PlayerConfig {
    walk_speed: 5.0,
    sprint_speed: 8.0,
    crouch_speed: 2.5,
    jump_velocity: 8.0,
    // ... see PlayerConfig fields for the full list
    ..default()
};
spawn_player(&mut commands, config, Vec3::new(0.0, 2.0, 0.0));
```

## Collision Layers

World geometry should use `GameLayer::World` to collide with the player:

```rust
use avian3d::prelude::*;
use player_controller::prelude::*;

commands.spawn((
    RigidBody::Static,
    Collider::cuboid(10.0, 1.0, 10.0),
    CollisionLayers::new(GameLayer::World, [GameLayer::Player]),
    // mesh, material, transform...
));
```

## Querying Player State

The player entity has marker components you can query:

```rust
fn my_system(query: Query<(&PlayerVelocity, &Transform, Has<Grounded>, Has<Sprinting>), With<Player>>) {
    let Ok((velocity, transform, grounded, sprinting)) = query.single() else { return };
    // ...
}
```

Available state markers: `Player`, `Grounded`, `Sprinting`, `Crouching`, `Sliding`.

## Running the Example

```sh
cargo run --example gymnasium
```

The gymnasium example includes slopes, jump gaps, obstacles, crouch tunnels, height jumps, and slide ramps for testing the controller.
