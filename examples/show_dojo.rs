//! Loads and renders a glTF file as a scene.

use bevy::prelude::*;
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};

fn main() {
    App::new()
        .add_plugin(NoCameraPlayerPlugin)
        .insert_resource(MovementSettings {
            speed: 1.0,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(FlyCam);

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 10.0),
        ..default()
    });

    commands.spawn(SceneBundle {
        scene: asset_server.load("matrix_dojo_replica.glb#Scene0"),
        ..default()
    });
}
