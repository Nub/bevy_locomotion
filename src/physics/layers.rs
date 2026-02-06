use avian3d::prelude::*;

/// Collision layers for the physics simulation
#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    Default,
    /// Player character
    Player,
    /// Static world geometry
    World,
    /// Triggers and sensors
    Trigger,
}
