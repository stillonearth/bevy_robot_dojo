#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![feature(let_chains)]

mod mujoco_parser;

use std::{cell::RefCell, rc::Rc};

use avian3d::prelude::{Joint, *};
use bevy::{
    color::palettes::css::*,
    pbr::wireframe::{WireframeColor, WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
};
use bevy_flycam::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use mujoco_parser::{Body, Geom, MuJoCoFile};

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
        .init_asset::<mujoco_parser::MuJoCoFile>()
        .init_asset_loader::<mujoco_parser::MuJoCoFileLoader>()
        .add_systems(Startup, setup)
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mujoco_handle: Handle<MuJoCoFile> = asset_server.load("ant.xml");
    commands.insert_resource(MuJoCoFileHandle(mujoco_handle));

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
        // FlyCam,
    ));
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Loading,
    Simulation,
}

#[derive(Component)]
struct MuJoCoRoot;

#[derive(Resource)]
struct MuJoCoFileHandle(Handle<MuJoCoFile>);

#[derive(Component)]
struct GeomWrapper;

fn spawn_mujoco_model(
    commands: Commands,
    rpy_assets: Res<Assets<MuJoCoFile>>,
    mujoco_handle: Res<MuJoCoFileHandle>,
    mut app_state: ResMut<NextState<AppState>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
) {
    let mujoco_file = rpy_assets.get(mujoco_handle.0.id());
    if mujoco_file.is_none() {
        return;
    }

    app_state.set(AppState::Simulation);

    // Closure that can call itself recursively
    struct SpawnEntities<'s> {
        f: &'s dyn Fn(&SpawnEntities, Body, &mut ChildBuilder, usize),
    }

    impl SpawnEntities<'_> {
        /// Spawn a bevy entity for MuJoCo body
        #[allow(clippy::too_many_arguments)]
        fn spawn_body(
            &self,
            child_builder: &mut ChildBuilder,
            body: &Body,
            meshes: &Rc<RefCell<ResMut<Assets<Mesh>>>>,
            materials: &Rc<RefCell<ResMut<Assets<StandardMaterial>>>>,
            images: &Rc<RefCell<ResMut<Assets<Image>>>>,
            add_children: impl FnOnce(&mut ChildBuilder),
            depth: usize,
        ) {
            if depth == 2 {
                return;
            }

            let mut binding_1: EntityCommands;
            {
                // let mut commands = commands.borrow_mut();
                let materials = materials.borrow_mut();
                let mut meshes = meshes.borrow_mut();
                let images = images.borrow_mut();

                let body_name = body.name.clone().unwrap_or_default();

                binding_1 = child_builder.spawn((
                    Name::new(format!("MuJoCo::body_{}", body_name.as_str())),
                    body.transform(),
                    body.clone(),
                ));

                if let Some(joint) = body.joint.clone() {
                    binding_1.insert(joint);
                }

                if body.joint.is_some() {
                    let joint = body.clone().joint.unwrap();
                    binding_1.insert(joint);
                } else {
                    let joint = crate::mujoco_parser::Joint {
                        joint_type: "none".to_string(),
                        pos: (0.0, 0.0, 0.0),
                        axis: None,
                        range: None,
                        name: None,
                        margin: None,
                    };
                    binding_1.insert(joint);
                }

                // bind

                binding_1.with_children(|children| {
                    let mut cmd = children.spawn((
                        Mesh3d(meshes.add(body.geom.mesh())),
                        body.geom.transform(),
                        WireframeColor { color: LIME.into() },
                        Name::new(format!(
                            "MuJoCo::mesh_{}",
                            body.name.clone().unwrap_or_default().as_str()
                        )),
                        body.clone().geom,
                    ));

                    if body.joint.is_some() {
                        cmd.insert((
                            // RigidBody::Dynamic,
                            body.geom.mass_properties_bundle(),
                            // body.geom.collider(),
                        ));
                    } else {
                        cmd.insert((
                            // RigidBody::Dynamic,
                            body.geom.mass_properties_bundle(),
                            // body.geom.collider(),
                        ));
                    }
                });
            }

            binding_1.with_children(add_children);
        }
    }

    let meshes = Rc::new(RefCell::new(meshes));
    let materials = Rc::new(RefCell::new(materials));
    let images = Rc::new(RefCell::new(images));
    let commands = Rc::new(RefCell::new(commands));

    let spawn_entities = SpawnEntities {
        f: &|func, body, child_builder, depth| {
            let add_children = |child_builder: &mut ChildBuilder| {
                for child in body.clone().children {
                    (func.f)(func, child, child_builder, depth + 1);
                }
            };

            func.spawn_body(
                child_builder,
                &body.clone(),
                &meshes,
                &materials,
                &images,
                add_children,
                depth,
            );
        },
    };

    let mut commands = commands.borrow_mut();
    let bodies = mujoco_file.unwrap().0.clone();

    let mut binding = commands.spawn((Name::new("MuJoCo::world"), Transform::IDENTITY, MuJoCoRoot));

    binding.with_children(|child_builder| {
        for body in bodies {
            (spawn_entities.f)(&spawn_entities, body, child_builder, 0);
        }
    });
}

fn spawn_mujoco_joints(
    mut commands: Commands,
    // q_mujoco_root: Query<(Entity, &MuJoCoRoot)>,
    q_joints: Query<(Entity, &Parent, &mujoco_parser::Joint)>,
    q_geoms: Query<(Entity, &Parent, &Geom)>,
) {
    // iterate over joints and inser avian joint
    for (entity, p1, joint) in q_joints.iter() {
        // handle "none" joints
        if joint.joint_type == "none" {
            // find upper mesh
            let parent_geom = q_geoms.iter().find(|(_, p2, _)| p2.get() == p1.get());
            if parent_geom.is_none() {
                continue;
            }
            let (parent_entity, _, _) = parent_geom.unwrap();
            let child_geom = q_geoms.iter().find(|(_, p2, _)| p2.get() == entity);
            if parent_geom.is_none() {
                continue;
            }
            let (child_entity, _, _) = child_geom.unwrap();

            commands.spawn((
                FixedJoint::new(child_entity, parent_entity),
                Name::new("Joint"),
            ));
            println!("spawned joint");
            commands.spawn(FixedJoint::new(parent_entity, entity));
        }
    }
}
