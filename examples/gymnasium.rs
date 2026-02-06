use avian3d::prelude::*;
use bevy::{prelude::*, window::{CursorGrabMode, CursorOptions, PrimaryWindow}};
use bevy_locomotion::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "FPS Character Controller".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BevyLocomotionPlugin)
        .init_resource::<JumpTracker>()
        .add_systems(Startup, (setup, spawn_hud, setup_cursor_grab))
        .add_systems(Update, toggle_cursor_grab);

    #[cfg(feature = "gym-audio")]
    app.add_systems(Startup, gym_audio::load_audio)
        .add_systems(Update, gym_audio::play_audio);

    app.add_systems(Update, (update_screen_labels, update_hud))
        .run();
}

fn setup(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
) {
    spawn_player(&mut commands, PlayerConfig::default(), Vec3::new(0.0, 2.0, 0.0));
    spawn_gymnasium(commands, meshes, materials, images);
}

// ── HUD ─────────────────────────────────────────────────────────────

#[derive(Component)]
struct HudText;

/// Tracks jump height: records Y when leaving ground, tracks peak
#[derive(Resource, Default)]
struct JumpTracker {
    start_y: f32,
    peak_y: f32,
    last_jump_height: f32,
    was_grounded: bool,
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        HudText,
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
    ));
}

fn update_hud(
    player_query: Query<(&PlayerVelocity, &Transform, Has<Grounded>), With<Player>>,
    mut hud_query: Query<&mut Text, With<HudText>>,
    mut tracker: ResMut<JumpTracker>,
) {
    let Ok((velocity, transform, grounded)) = player_query.single() else {
        return;
    };

    let y = transform.translation.y;
    let horizontal_speed = Vec2::new(velocity.x, velocity.z).length();

    // Track jump height
    if grounded && !tracker.was_grounded {
        // Just landed — record the jump height
        tracker.last_jump_height = tracker.peak_y - tracker.start_y;
    }
    if !grounded && tracker.was_grounded {
        // Just left ground
        tracker.start_y = y;
        tracker.peak_y = y;
    }
    if !grounded {
        tracker.peak_y = tracker.peak_y.max(y);
    }
    tracker.was_grounded = grounded;

    for mut text in &mut hud_query {
        **text = format!(
            "Speed: {:.1} m/s\nJump:  {:.2} m",
            horizontal_speed, tracker.last_jump_height,
        );
    }
}

// ── Screen-space label system ────────────────────────────────────────

/// A UI label that tracks a world-space position
#[derive(Component)]
struct ScreenLabel {
    world_pos: Vec3,
}

/// Projects world positions to screen space and positions UI labels
fn update_screen_labels(
    camera_query: Query<(&Camera, &GlobalTransform), With<FpsCamera>>,
    mut label_query: Query<(&mut Node, &mut Visibility, &ScreenLabel)>,
) {
    let Ok((camera, camera_gt)) = camera_query.single() else {
        return;
    };

    for (mut node, mut vis, label) in &mut label_query {
        let distance = camera_gt.translation().distance(label.world_pos);

        if distance > 50.0 {
            *vis = Visibility::Hidden;
            continue;
        }

        match camera.world_to_viewport(camera_gt, label.world_pos) {
            Ok(vp) => {
                *vis = Visibility::Inherited;
                node.left = Val::Px(vp.x - 30.0);
                node.top = Val::Px(vp.y - 12.0);
            }
            Err(_) => {
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// Spawns a screen-space label that tracks a world position
fn spawn_label(commands: &mut Commands, text: &str, world_pos: Vec3) {
    commands.spawn((
        Text::new(text),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
            ..default()
        },
        ScreenLabel { world_pos },
    ));
}

// ── Audio ───────────────────────────────────────────────────────────

#[cfg(feature = "gym-audio")]
mod gym_audio {
    use bevy::prelude::*;
    use bevy_locomotion::prelude::*;

    #[derive(Resource)]
    pub struct AudioHandles {
        footstep: Handle<AudioSource>,
        land: Handle<AudioSource>,
        jump: Handle<AudioSource>,
        slide_start: Handle<AudioSource>,
        slide_end: Handle<AudioSource>,
        ledge_grab: Handle<AudioSource>,
        ledge_climb_start: Handle<AudioSource>,
        ledge_climb_finish: Handle<AudioSource>,
        wall_jump: Handle<AudioSource>,
        step_up: Handle<AudioSource>,
    }

    pub fn load_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.insert_resource(AudioHandles {
            footstep: asset_server.load("audio/footstep.ogg"),
            land: asset_server.load("audio/land.ogg"),
            jump: asset_server.load("audio/jump.ogg"),
            slide_start: asset_server.load("audio/slide_start.ogg"),
            slide_end: asset_server.load("audio/slide_end.ogg"),
            ledge_grab: asset_server.load("audio/ledge_grab.ogg"),
            ledge_climb_start: asset_server.load("audio/ledge_climb_start.ogg"),
            ledge_climb_finish: asset_server.load("audio/ledge_climb_finish.ogg"),
            wall_jump: asset_server.load("audio/wall_jump.ogg"),
            step_up: asset_server.load("audio/step_up.ogg"),
        });
    }

    pub fn play_audio(
        mut commands: Commands,
        mut reader: MessageReader<PlayerAudioMessage>,
        handles: Option<Res<AudioHandles>>,
    ) {
        let Some(handles) = handles else { return };

        for msg in reader.read() {
            let (handle, volume) = match msg {
                PlayerAudioMessage::Footstep { speed } => {
                    let vol = (speed / 8.0).clamp(0.3, 1.0);
                    (handles.footstep.clone(), vol)
                }
                PlayerAudioMessage::Landed { impact_speed } => {
                    let vol = (impact_speed / 15.0).clamp(0.4, 1.0);
                    (handles.land.clone(), vol)
                }
                PlayerAudioMessage::Jumped => (handles.jump.clone(), 0.6),
                PlayerAudioMessage::SlideStart => (handles.slide_start.clone(), 0.7),
                PlayerAudioMessage::SlideEnd => (handles.slide_end.clone(), 0.5),
                PlayerAudioMessage::LedgeGrabbed => (handles.ledge_grab.clone(), 0.7),
                PlayerAudioMessage::LedgeClimbStarted => (handles.ledge_climb_start.clone(), 0.6),
                PlayerAudioMessage::LedgeClimbFinished => (handles.ledge_climb_finish.clone(), 0.7),
                PlayerAudioMessage::WallJumped => (handles.wall_jump.clone(), 0.7),
                PlayerAudioMessage::SteppedUp => (handles.step_up.clone(), 0.4),
                PlayerAudioMessage::LadderEnter => (handles.step_up.clone(), 0.5),
                PlayerAudioMessage::LadderExit => (handles.step_up.clone(), 0.4),
                PlayerAudioMessage::ForcedSlideStart => (handles.slide_start.clone(), 0.6),
                PlayerAudioMessage::ForcedSlideEnd => (handles.slide_end.clone(), 0.4),
            };

            commands.spawn((
                AudioPlayer::new(handle),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Despawn,
                    volume: bevy::audio::Volume::Linear(volume),
                    ..default()
                },
            ));

            info!("{msg:?}");
        }
    }
}

// ── Checker texture ──────────────────────────────────────────────────

fn create_checker_image() -> Image {
    let size = 64usize;
    let check_size = 8;
    let mut data = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let checker = ((x / check_size) + (y / check_size)) % 2 == 0;
            let idx = (y * size + x) * 4;
            let (r, g, b) = if checker {
                (180u8, 200u8, 170u8)
            } else {
                (140u8, 160u8, 130u8)
            };
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    Image::new(
        bevy::render::render_resource::Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

// ── Material helpers ─────────────────────────────────────────────────

fn ramp_color(degrees: f32) -> Color {
    // Green at 10° → yellow at 30° → orange at 45° → red at 60°
    let t = ((degrees - 10.0) / 50.0).clamp(0.0, 1.0);
    if t < 0.5 {
        let u = t * 2.0;
        Color::srgb(0.4 + u * 0.4, 0.7 - u * 0.2, 0.4 - u * 0.2)
    } else {
        let u = (t - 0.5) * 2.0;
        Color::srgb(0.8 + u * 0.1, 0.5 - u * 0.3, 0.2 - u * 0.1)
    }
}

// ── Gymnasium ────────────────────────────────────────────────────────

fn spawn_gymnasium(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let checker = images.add(create_checker_image());

    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.55, 0.35),
        base_color_texture: Some(checker),
        perceptual_roughness: 0.9,
        ..default()
    });
    let stone_a = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.36, 0.40),
        perceptual_roughness: 0.85,
        ..default()
    });
    let stone_b = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.50, 0.48),
        perceptual_roughness: 0.8,
        ..default()
    });
    let accent = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.6),
        perceptual_roughness: 0.5,
        metallic: 0.3,
        ..default()
    });
    let ceiling_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.3, 0.3),
        perceptual_roughness: 0.9,
        ..default()
    });

    // ── Ground ───────────────────────────────────────────────────
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 200.0))),
        MeshMaterial3d(ground_mat),
        Transform::from_translation(Vec3::ZERO),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
        CollisionLayers::new(GameLayer::World, [GameLayer::Player]),
    ));

    // ══════════════════════════════════════════════════════════════
    // Layout: each section in its own row along Z, all items
    // expanding in the +X direction from X = 5.
    //
    //   Z =  48  SLOPES        (ramps face +Z uphill, extend to ~60)
    //   Z =  38  LEDGE GRAB    (walls)
    //   Z =  30  LADDERS       (walls + sensor volumes)
    //   Z =  20  JUMPS         (platforms with gaps)
    //   Z =  10  OBSTACLES     (step-over walls)
    //   Z =  -8  HEIGHT JUMPS  (elevation pairs)
    //   Z = -18  CROUCH        (tunnels extend +Z to ~-12)
    //   Z = -30  SLIDES        (downhill ramps, extend ±8)
    //   Z = -50  FORCED SLIDES (ramps face +Z uphill, extend to ~-38)
    // ══════════════════════════════════════════════════════════════

    // ══════════════════════════════════════════════════════════════
    // SLOPE GALLERY  (Z = 48)
    // Ramps from 10° to 60° in 5° steps, expanding +X
    // ══════════════════════════════════════════════════════════════

    let slope_angles: &[f32] = &[10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 60.0];
    let slope_base_x = 5.0;
    let slope_base_z = 48.0;
    let slope_spacing = 7.0;

    for (i, &deg) in slope_angles.iter().enumerate() {
        let x = slope_base_x + (i as f32) * slope_spacing;
        let rad = deg.to_radians();
        let ramp_len = 12.0;
        let ramp_rise = (ramp_len / 2.0) * rad.sin();

        let mat = materials.add(StandardMaterial {
            base_color: ramp_color(deg),
            perceptual_roughness: 0.7,
            ..default()
        });

        spawn_ramp(
            &mut commands, &mut meshes, mat,
            Vec3::new(5.0, 0.25, ramp_len),
            Vec3::new(x, ramp_rise, slope_base_z + ramp_len / 2.0),
            rad,
        );

        spawn_label(&mut commands, &format!("{deg}°"), Vec3::new(x, 1.5, slope_base_z));
    }

    spawn_label(&mut commands, "SLOPES", Vec3::new(0.0, 2.5, slope_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // LEDGE GRAB  (Z = 38)
    // Walls at various heights for testing ledge detection & climb
    // ══════════════════════════════════════════════════════════════

    let ledge_heights: &[f32] = &[1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
    let ledge_base_x = 5.0;
    let ledge_base_z = 38.0;
    let ledge_spacing = 5.0;

    for (i, &h) in ledge_heights.iter().enumerate() {
        let x = ledge_base_x + (i as f32) * ledge_spacing;
        let mat = if i % 2 == 0 { stone_a.clone() } else { stone_b.clone() };

        // Thick wall to grab onto (with LedgeGrabbable marker)
        let size = Vec3::new(3.0, h, 1.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(x, h / 2.0, ledge_base_z)),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            CollisionLayers::new(GameLayer::World, [GameLayer::Player]),
            LedgeGrabbable,
        ));

        spawn_label(&mut commands, &format!("{h}m"), Vec3::new(x, h + 0.5, ledge_base_z));
    }

    spawn_label(&mut commands, "LEDGE GRAB", Vec3::new(0.0, 5.0, ledge_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // LADDERS  (Z = 30)
    // Walls with sensor ladder volumes for climbing
    // ══════════════════════════════════════════════════════════════

    let ladder_heights: &[f32] = &[4.0, 6.0, 8.0];
    let ladder_base_x = 5.0;
    let ladder_base_z = 30.0;
    let ladder_spacing = 6.0;

    let ladder_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.45, 0.30),
        perceptual_roughness: 0.9,
        ..default()
    });

    for (i, &h) in ladder_heights.iter().enumerate() {
        let x = ladder_base_x + (i as f32) * ladder_spacing;

        // Back wall
        spawn_box(
            &mut commands, &mut meshes, stone_a.clone(),
            Vec3::new(3.0, h, 0.4),
            Vec3::new(x, h / 2.0, ladder_base_z),
        );

        // Ladder sensor volume (slightly in front of wall)
        let ladder_size = Vec3::new(1.0, h, 0.3);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(ladder_size.x, ladder_size.y, ladder_size.z))),
            MeshMaterial3d(ladder_mat.clone()),
            Transform::from_translation(Vec3::new(x, h / 2.0, ladder_base_z - 0.35)),
            RigidBody::Static,
            Collider::cuboid(ladder_size.x, ladder_size.y, ladder_size.z),
            CollisionLayers::new(GameLayer::Trigger, [GameLayer::Player]),
            Sensor,
            Ladder,
        ));

        // Platform on top
        spawn_box(
            &mut commands, &mut meshes, stone_b.clone(),
            Vec3::new(3.0, 0.3, 2.0),
            Vec3::new(x, h + 0.15, ladder_base_z + 1.0),
        );

        spawn_label(&mut commands, &format!("{h}m"), Vec3::new(x, h + 1.0, ladder_base_z - 1.5));
    }

    spawn_label(&mut commands, "LADDERS", Vec3::new(0.0, 9.0, ladder_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // JUMP COURSE  (Z = 20)
    // Platforms with increasing gap distances, expanding +X
    // ══════════════════════════════════════════════════════════════

    let jump_gaps: &[f32] = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let jump_start_x = 5.0;
    let platform_size = Vec3::new(3.0, 0.6, 3.0);
    let jump_z = 20.0;
    let jump_h = 0.3;

    let mut cursor_x = jump_start_x;

    for (i, &gap) in jump_gaps.iter().enumerate() {
        let mat = if i % 2 == 0 { stone_a.clone() } else { stone_b.clone() };
        spawn_box(&mut commands, &mut meshes, mat,
            platform_size,
            Vec3::new(cursor_x, jump_h, jump_z),
        );

        let label_x = cursor_x + platform_size.x / 2.0 + gap / 2.0;
        spawn_label(&mut commands, &format!("{gap}m gap"), Vec3::new(label_x, 1.5, jump_z));

        cursor_x += platform_size.x / 2.0 + gap + platform_size.x / 2.0;
    }
    // Final landing platform
    spawn_box(&mut commands, &mut meshes, accent.clone(),
        platform_size, Vec3::new(cursor_x, jump_h, jump_z));

    spawn_label(&mut commands, "JUMPS", Vec3::new(0.0, 2.5, jump_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // OBSTACLE COURSE  (Z = 10)
    // Walls of increasing height, expanding +X
    // ══════════════════════════════════════════════════════════════

    let wall_heights: &[f32] = &[0.3, 0.5, 0.7, 1.0, 1.3, 1.5, 1.8, 2.0, 2.5];
    let obstacle_base_x = 5.0;
    let obstacle_base_z = 10.0;
    let obstacle_spacing = 4.0;

    for (i, &h) in wall_heights.iter().enumerate() {
        let x = obstacle_base_x + (i as f32) * obstacle_spacing;
        let mat = if i % 2 == 0 { stone_a.clone() } else { stone_b.clone() };
        spawn_box(&mut commands, &mut meshes, mat,
            Vec3::new(2.0, h, 0.4),
            Vec3::new(x, h / 2.0, obstacle_base_z),
        );

        spawn_label(&mut commands, &format!("{h}m"), Vec3::new(x, h + 0.4, obstacle_base_z));
    }

    spawn_label(&mut commands, "OBSTACLES", Vec3::new(0.0, 3.5, obstacle_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // VARIABLE HEIGHT JUMPS  (Z = -8)
    // Same gap, different elevation changes, expanding +X
    // ══════════════════════════════════════════════════════════════

    let height_jumps: &[(f32, f32)] = &[
        (0.0, 1.0),   // jump up 1m
        (0.0, 2.0),   // jump up 2m
        (0.0, -1.0),  // drop 1m
        (0.0, -2.0),  // drop 2m
        (1.0, 2.0),   // up 1m
        (2.0, 1.0),   // down 1m
    ];
    let vj_base_x = 5.0;
    let vj_base_z = -8.0;

    let mut vj_x = vj_base_x;
    for (i, &(from_h, to_h)) in height_jumps.iter().enumerate() {
        let mat_from = if i % 2 == 0 { stone_a.clone() } else { stone_b.clone() };
        let mat_to = accent.clone();
        let gap = 3.0;

        spawn_box(&mut commands, &mut meshes, mat_from,
            Vec3::new(2.5, 0.5, 2.5),
            Vec3::new(vj_x, from_h + 0.25, vj_base_z),
        );
        spawn_box(&mut commands, &mut meshes, mat_to,
            Vec3::new(2.5, 0.5, 2.5),
            Vec3::new(vj_x + 2.5 + gap, to_h + 0.25, vj_base_z),
        );

        let diff = to_h - from_h;
        let sign = if diff >= 0.0 { "+" } else { "" };
        spawn_label(
            &mut commands,
            &format!("{sign}{diff}m"),
            Vec3::new(vj_x + (2.5 + gap) / 2.0, from_h.max(to_h) + 1.5, vj_base_z),
        );

        vj_x += 2.5 + gap + 2.5 + 3.0;
    }

    spawn_label(&mut commands, "HEIGHT JUMPS", Vec3::new(0.0, 4.0, vj_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // CROUCH TUNNELS  (Z = -18)
    // Corridors with decreasing ceiling clearance, expanding +X
    // ══════════════════════════════════════════════════════════════

    let clearances: &[f32] = &[1.8, 1.5, 1.2, 1.0, 0.8];
    let tunnel_base_x = 5.0;
    let tunnel_base_z = -18.0;
    let tunnel_spacing = 5.0;
    let tunnel_width = 3.0;
    let tunnel_depth = 6.0;

    for (i, &clearance) in clearances.iter().enumerate() {
        let x = tunnel_base_x + (i as f32) * tunnel_spacing;
        let floor_h = 0.3;

        // Floor
        spawn_box(&mut commands, &mut meshes, stone_a.clone(),
            Vec3::new(tunnel_width, floor_h, tunnel_depth),
            Vec3::new(x, floor_h / 2.0, tunnel_base_z),
        );

        // Ceiling
        let ceil_y = floor_h + clearance + 0.15;
        spawn_box(&mut commands, &mut meshes, ceiling_mat.clone(),
            Vec3::new(tunnel_width, 0.3, tunnel_depth),
            Vec3::new(x, ceil_y, tunnel_base_z),
        );

        // Side walls
        for side in [-1.0, 1.0] {
            spawn_box(&mut commands, &mut meshes, stone_b.clone(),
                Vec3::new(0.2, clearance + 0.5, tunnel_depth),
                Vec3::new(x + side * (tunnel_width / 2.0 + 0.1), (clearance + 0.5) / 2.0 + floor_h, tunnel_base_z),
            );
        }

        spawn_label(
            &mut commands,
            &format!("{clearance}m clear"),
            Vec3::new(x, ceil_y + 0.5, tunnel_base_z),
        );
    }

    spawn_label(&mut commands, "CROUCH", Vec3::new(0.0, 3.0, tunnel_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // SLIDE COURSE  (Z = -30)
    // Downhill ramps for sprint-slide testing, expanding +X
    // ══════════════════════════════════════════════════════════════

    let slide_angles: &[f32] = &[5.0, 10.0, 15.0, 20.0, 30.0];
    let slide_base_x = 5.0;
    let slide_base_z = -30.0;
    let slide_spacing = 8.0;

    for (i, &deg) in slide_angles.iter().enumerate() {
        let x = slide_base_x + (i as f32) * slide_spacing;
        let rad = deg.to_radians();
        let mat = materials.add(StandardMaterial {
            base_color: ramp_color(deg),
            perceptual_roughness: 0.6,
            ..default()
        });

        spawn_ramp(
            &mut commands, &mut meshes, mat,
            Vec3::new(4.0, 0.25, 16.0),
            Vec3::new(x, -0.5, slide_base_z),
            -rad, // downhill
        );

        spawn_label(&mut commands, &format!("-{deg}° slide"), Vec3::new(x, 1.5, slide_base_z + 9.0));
    }

    spawn_label(&mut commands, "SLIDES", Vec3::new(0.0, 3.0, slide_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // FORCED SLIDES  (Z = -50)
    // Ramps with ForceSlide marker that push the player downhill
    // ══════════════════════════════════════════════════════════════

    let fslide_angles: &[f32] = &[15.0, 25.0, 35.0, 45.0];
    let fslide_base_x = 5.0;
    let fslide_base_z = -50.0;
    let fslide_spacing = 8.0;

    for (i, &deg) in fslide_angles.iter().enumerate() {
        let x = fslide_base_x + (i as f32) * fslide_spacing;
        let rad = deg.to_radians();
        let ramp_len = 12.0;
        let ramp_rise = (ramp_len / 2.0) * rad.sin();

        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.3, 0.3),
            perceptual_roughness: 0.6,
            ..default()
        });

        // Ramp with ForceSlide marker
        let size = Vec3::new(5.0, 0.25, ramp_len);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(x, ramp_rise, fslide_base_z + ramp_len / 2.0))
                .with_rotation(Quat::from_rotation_x(rad)),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            CollisionLayers::new(GameLayer::World, [GameLayer::Player]),
            ForceSlide,
        ));

        spawn_label(
            &mut commands,
            &format!("{deg}° slide"),
            Vec3::new(x, 1.5, fslide_base_z),
        );
    }

    spawn_label(&mut commands, "FORCED SLIDES", Vec3::new(0.0, 4.0, fslide_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // LIGHTING
    // ══════════════════════════════════════════════════════════════

    commands.spawn((
        DirectionalLight {
            illuminance: 14000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.5, 0.0)),
    ));

    commands.spawn(AmbientLight {
        color: Color::srgb(0.6, 0.7, 0.9),
        brightness: 350.0,
        affects_lightmapped_meshes: true,
    });
}

// ── Geometry helpers ─────────────────────────────────────────────────

fn spawn_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    size: Vec3,
    position: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
        CollisionLayers::new(GameLayer::World, [GameLayer::Player]),
    ));
}

fn spawn_ramp(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    size: Vec3,
    position: Vec3,
    angle: f32,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(material),
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_x(angle)),
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, size.z),
        CollisionLayers::new(GameLayer::World, [GameLayer::Player]),
    ));
}

// ── Cursor grab ──────────────────────────────────────────────────────

fn setup_cursor_grab(mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor_query.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn toggle_cursor_grab(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(mut cursor) = cursor_query.single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if mouse.just_pressed(MouseButton::Left) && cursor.grab_mode == CursorGrabMode::None {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}
