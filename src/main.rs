#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![feature(let_chains)]

mod mujoco_plugin;
mod mujoco_xml_parser;
mod physics;

use avian3d::{
    prelude::{Collider, RigidBody},
    PhysicsPlugins,
};
use bevy::{
    color::palettes::css::*,
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
};
use bevy_flycam::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::mujoco_plugin::*;
use crate::mujoco_xml_parser::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            }),
            WireframePlugin,
            PhysicsPlugins::default(),
            WorldInspectorPlugin::new(),
        ))
        .insert_resource(WireframeConfig {
            global: true,
            default_color: WHITE.into(),
        })
        // .insert_resource(SubstepCount(50))
        .init_asset::<mujoco_xml_parser::MuJoCoFile>()
        .init_asset_loader::<mujoco_xml_parser::MuJoCoFileLoader>()
        .add_systems(
            Startup,
            (setup_scene, setup_mujoco_robot.after(setup_scene)),
        )
        .add_systems(
            Update,
            (
                spawn_mujoco_model,
                spawn_mujoco_joints.after(spawn_mujoco_model),
            )
                .run_if(in_state(AppState::Loading)),
        )
        // .add_systems(Update, ().run_if(in_state(AppState::Simulation)))
        .init_state::<AppState>()
        .add_plugins(NoCameraPlayerPlugin)
        .insert_resource(MovementSettings {
            speed: 2.0,
            ..default()
        })
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // ground plane
    let cube_mesh = meshes.add(Cuboid::default());
    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(materials.add(Color::srgb(0.7, 0.7, 0.8))),
        Transform::from_xyz(0.0, -2.0, 0.0).with_scale(Vec3::new(100.0, 1.0, 100.0)),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.5, 7.5, 3.5).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        FlyCam,
    ));
}

fn setup_mujoco_robot(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mujoco_handle: Handle<MuJoCoFile> = asset_server.load("ant.xml");
    commands.insert_resource(mujoco_plugin::MuJoCoFileHandle(mujoco_handle));
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Loading,
    Simulation,
}
