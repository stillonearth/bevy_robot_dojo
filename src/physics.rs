use avian3d::prelude::PhysicsLayer;

#[allow(dead_code)]
#[derive(PhysicsLayer, Default)]
enum GameLayer {
    #[default]
    Default, // Layer 0 - the default layer that objects are assigned to
    Player, // Layer 1
    Enemy,  // Layer 2
    Ground, // Layer 3
}
