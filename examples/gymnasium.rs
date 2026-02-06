use avian3d::prelude::*;
use bevy::prelude::*;
use player_controller::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "FPS Character Controller".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PlayerControllerPlugin)
        .init_resource::<JumpTracker>()
        .add_systems(Startup, (setup, spawn_hud))
        .add_systems(Update, (update_screen_labels, update_hud))
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
        Mesh3d(meshes.add(Plane3d::default().mesh().size(120.0, 120.0))),
        MeshMaterial3d(ground_mat),
        Transform::from_translation(Vec3::ZERO),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
        CollisionLayers::new(GameLayer::World, [GameLayer::Player]),
    ));

    // ══════════════════════════════════════════════════════════════
    // SLOPE GALLERY  (north, +Z)
    // Ramps from 10° to 60° in 5° steps
    // ══════════════════════════════════════════════════════════════

    let slope_angles: &[f32] = &[10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 60.0];
    let slope_base_z = 12.0;
    let slope_spacing = 7.0;

    for (i, &deg) in slope_angles.iter().enumerate() {
        let x = (i as f32 - (slope_angles.len() as f32 - 1.0) / 2.0) * slope_spacing;
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

        // Label at base of ramp
        spawn_label(&mut commands, &format!("{deg}°"), Vec3::new(x, 1.5, slope_base_z));
    }

    // Section sign
    spawn_label(&mut commands, "SLOPES", Vec3::new(0.0, 2.5, slope_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // JUMP COURSE  (east, +X)
    // Platforms with increasing gap distances
    // ══════════════════════════════════════════════════════════════

    let jump_gaps: &[f32] = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let jump_start_x = 10.0;
    let platform_size = Vec3::new(3.0, 0.6, 3.0);
    let jump_z = 0.0;
    let jump_h = 0.3; // platform center y

    let mut cursor_x = jump_start_x;

    for (i, &gap) in jump_gaps.iter().enumerate() {
        let mat = if i % 2 == 0 { stone_a.clone() } else { stone_b.clone() };
        spawn_box(&mut commands, &mut meshes, mat,
            platform_size,
            Vec3::new(cursor_x, jump_h, jump_z),
        );

        // Label the gap (between this platform and the next)
        let label_x = cursor_x + platform_size.x / 2.0 + gap / 2.0;
        spawn_label(&mut commands, &format!("{gap}m gap"), Vec3::new(label_x, 1.5, jump_z));

        cursor_x += platform_size.x / 2.0 + gap + platform_size.x / 2.0;
    }
    // Final landing platform
    spawn_box(&mut commands, &mut meshes, accent.clone(),
        platform_size, Vec3::new(cursor_x, jump_h, jump_z));

    spawn_label(&mut commands, "JUMPS", Vec3::new(jump_start_x, 2.5, jump_z - 3.0));

    // ══════════════════════════════════════════════════════════════
    // OBSTACLE COURSE  (south, -Z)
    // Walls of increasing height
    // ══════════════════════════════════════════════════════════════

    let wall_heights: &[f32] = &[0.3, 0.5, 0.7, 1.0, 1.3, 1.5, 1.8, 2.0, 2.5];
    let obstacle_base_z = -10.0;
    let obstacle_spacing = 4.0;

    for (i, &h) in wall_heights.iter().enumerate() {
        let x = (i as f32 - (wall_heights.len() as f32 - 1.0) / 2.0) * obstacle_spacing;
        let mat = if i % 2 == 0 { stone_a.clone() } else { stone_b.clone() };
        spawn_box(&mut commands, &mut meshes, mat,
            Vec3::new(2.0, h, 0.4),
            Vec3::new(x, h / 2.0, obstacle_base_z),
        );

        spawn_label(&mut commands, &format!("{h}m"), Vec3::new(x, h + 0.4, obstacle_base_z));
    }

    spawn_label(&mut commands, "OBSTACLES", Vec3::new(0.0, 3.5, obstacle_base_z - 2.0));

    // ══════════════════════════════════════════════════════════════
    // CROUCH TUNNELS  (west, -X)
    // Corridors with decreasing ceiling clearance
    // ══════════════════════════════════════════════════════════════

    let clearances: &[f32] = &[1.8, 1.5, 1.2, 1.0, 0.8];
    let tunnel_start_x = -8.0;
    let tunnel_z = 0.0;
    let tunnel_width = 3.0;
    let tunnel_depth = 6.0;

    for (i, &clearance) in clearances.iter().enumerate() {
        let z = tunnel_z + (i as f32) * (tunnel_depth + 1.0);
        let floor_h = 0.3;

        // Floor
        spawn_box(&mut commands, &mut meshes, stone_a.clone(),
            Vec3::new(tunnel_width, floor_h, tunnel_depth),
            Vec3::new(tunnel_start_x, floor_h / 2.0, z),
        );

        // Ceiling
        let ceil_y = floor_h + clearance + 0.15;
        spawn_box(&mut commands, &mut meshes, ceiling_mat.clone(),
            Vec3::new(tunnel_width, 0.3, tunnel_depth),
            Vec3::new(tunnel_start_x, ceil_y, z),
        );

        // Side walls
        for side in [-1.0, 1.0] {
            spawn_box(&mut commands, &mut meshes, stone_b.clone(),
                Vec3::new(0.2, clearance + 0.5, tunnel_depth),
                Vec3::new(tunnel_start_x + side * (tunnel_width / 2.0 + 0.1), (clearance + 0.5) / 2.0 + floor_h, z),
            );
        }

        spawn_label(
            &mut commands,
            &format!("{clearance}m clear"),
            Vec3::new(tunnel_start_x, ceil_y + 0.5, z),
        );
    }

    spawn_label(&mut commands, "CROUCH", Vec3::new(tunnel_start_x, 3.0, tunnel_z - 4.0));

    // ══════════════════════════════════════════════════════════════
    // VARIABLE HEIGHT JUMPS  (southeast)
    // Same gap, different elevation changes
    // ══════════════════════════════════════════════════════════════

    let height_jumps: &[(f32, f32)] = &[
        (0.0, 1.0),   // jump up 1m
        (0.0, 2.0),   // jump up 2m
        (0.0, -1.0),  // drop 1m
        (0.0, -2.0),  // drop 2m
        (1.0, 2.0),   // up 1m
        (2.0, 1.0),   // down 1m
    ];
    let vj_base_x = 12.0;
    let vj_base_z = -20.0;

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

    spawn_label(&mut commands, "HEIGHT JUMPS", Vec3::new(vj_base_x + 15.0, 4.0, vj_base_z - 3.0));

    // ══════════════════════════════════════════════════════════════
    // SLIDE COURSE  (southwest, -X -Z)
    // Downhill ramps for sprint-slide testing
    // ══════════════════════════════════════════════════════════════

    let slide_angles: &[f32] = &[5.0, 10.0, 15.0, 20.0, 30.0];
    let slide_base_z = -20.0;
    let slide_base_x = -10.0;

    for (i, &deg) in slide_angles.iter().enumerate() {
        let z = slide_base_z - (i as f32) * 8.0;
        let rad = deg.to_radians();
        let mat = materials.add(StandardMaterial {
            base_color: ramp_color(deg),
            perceptual_roughness: 0.6,
            ..default()
        });

        spawn_ramp(
            &mut commands, &mut meshes, mat,
            Vec3::new(4.0, 0.25, 16.0),
            Vec3::new(slide_base_x, -0.5, z),
            -rad, // downhill
        );

        spawn_label(&mut commands, &format!("-{deg}° slide"), Vec3::new(slide_base_x, 1.5, z + 9.0));
    }

    spawn_label(&mut commands, "SLIDES", Vec3::new(slide_base_x, 3.0, slide_base_z + 5.0));

    // ══════════════════════════════════════════════════════════════
    // LEDGE GRAB  (northeast, +X +Z)
    // Walls at various heights for testing ledge detection & climb
    // ══════════════════════════════════════════════════════════════

    let ledge_heights: &[f32] = &[1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
    let ledge_base_x = 10.0;
    let ledge_base_z = 15.0;
    let ledge_spacing = 5.0;

    for (i, &h) in ledge_heights.iter().enumerate() {
        let x = ledge_base_x + (i as f32) * ledge_spacing;
        let mat = if i % 2 == 0 { stone_a.clone() } else { stone_b.clone() };

        // Thick wall to grab onto
        spawn_box(
            &mut commands, &mut meshes, mat,
            Vec3::new(3.0, h, 1.0),
            Vec3::new(x, h / 2.0, ledge_base_z),
        );

        spawn_label(&mut commands, &format!("{h}m"), Vec3::new(x, h + 0.5, ledge_base_z));
    }

    spawn_label(&mut commands, "LEDGE GRAB", Vec3::new(ledge_base_x + 12.0, 5.0, ledge_base_z - 3.0));

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
