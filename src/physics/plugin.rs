use avian3d::prelude::*;
use bevy::prelude::*;

/// Plugin that sets up the Avian3D physics engine
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            PhysicsPlugins::default()
                .with_length_unit(1.0), // 1 unit = 1 meter
        );

        // Configure gravity
        app.insert_resource(Gravity(Vec3::NEG_Y * 20.0)); // Slightly higher for snappy feel
    }
}
