#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![feature(let_chains)]

mod mujoco_parser;

use std::{cell::RefCell, rc::Rc};

use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::*,
    pbr::wireframe::{NoWireframe, WireframeColor, WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
    state::commands,
};
use bevy_flycam::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use mujoco_parser::{Body, MuJoCoFile};

use trees::Tree;

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
            (spawn_mujoco_model,).run_if(in_state(AppState::Loading)),
        )
        // .add_systems(Update, ().run_if(in_state(AppState::Simulation)))
        .init_state::<AppState>()
        // .add_plugins(NoCameraPlayerPlugin)
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
        Transform::from_xyz(5., 9., 18.).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        // FlyCam,
    ));
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Loading,
    Simulation,
}

#[derive(Deref, DerefMut)]
pub struct BodyTree(pub Tree<Body>);

#[derive(Resource)]
struct MuJoCoFileHandle(Handle<MuJoCoFile>);

fn spawn_mujoco_model(
    mut commands: Commands,
    rpy_assets: Res<Assets<MuJoCoFile>>,
    mujoco_handle: Res<MuJoCoFileHandle>,
    mut app_state: ResMut<NextState<AppState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
    // mujoco: ResMut<MuJoCoSimulation>,
) {
    let mujoco_file = rpy_assets.get(mujoco_handle.0.id());
    if mujoco_file.is_none() {
        return;
    }

    app_state.set(AppState::Simulation);

    // Closure that can call itself recursively
    struct SpawnEntities<'s> {
        f: &'s dyn Fn(&SpawnEntities, Option<Body>, Body, &mut ChildBuilder, usize),
    }

    impl SpawnEntities<'_> {
        /// Spawn a bevy entity for MuJoCo body
        #[allow(clippy::too_many_arguments)]
        fn spawn_body(
            &self,
            child_builder: &mut ChildBuilder,
            parent_body: Option<Body>,
            body: &Body,
            meshes: &Rc<RefCell<ResMut<Assets<Mesh>>>>,
            materials: &Rc<RefCell<ResMut<Assets<StandardMaterial>>>>,
            images: &Rc<RefCell<ResMut<Assets<Image>>>>,
            add_children: impl FnOnce(&mut ChildBuilder),
        ) {
            let mut binding: EntityCommands;
            {
                let mut materials = materials.borrow_mut();
                let mut meshes = meshes.borrow_mut();
                let mut images = images.borrow_mut();

                let body_name = body.name.clone().unwrap_or_default();

                binding = child_builder.spawn((
                    Name::new(format!("MuJoCo::body_{}", body_name.as_str())),
                    body.transform(),
                ));

                if let Some(joint) = body.joint.clone() {
                    binding.insert(joint);
                }

                binding.with_children(|children| {
                    let mut cmd = children.spawn((
                        Mesh3d(meshes.add(body.geom.mesh())),
                        body.geom.transform(),
                        WireframeColor { color: LIME.into() },
                        Name::new(format!(
                            "MuJoCo::mesh_{}",
                            body.name.clone().unwrap_or_default().as_str()
                        )),
                    ));

                    if parent_body.is_none() || parent_body.unwrap().joint.is_some() {
                        cmd.insert((
                            RigidBody::Dynamic,
                            body.geom.mass_properties_bundle(),
                            body.geom.collider(),
                        ));
                    }
                });
            }

            binding.with_children(add_children);
        }
    }

    let meshes = Rc::new(RefCell::new(meshes));
    let materials = Rc::new(RefCell::new(materials));
    let images = Rc::new(RefCell::new(images));
    let commands = Rc::new(RefCell::new(commands));

    let spawn_entities = SpawnEntities {
        f: &|func, parent_body, body, child_builder, depth| {
            let add_children = |child_builder: &mut ChildBuilder| {
                for child in body.clone().children {
                    (func.f)(func, Some(body.clone()), child, child_builder, depth + 1);
                }
            };

            func.spawn_body(
                child_builder,
                parent_body.clone(),
                &body.clone(),
                &meshes,
                &materials,
                &images,
                add_children,
            );
        },
    };

    let mut commands = commands.borrow_mut();
    let bodies = mujoco_file.unwrap().0.clone();
    commands
        .spawn((Name::new("MuJoCo::world"), Transform::IDENTITY))
        .with_children(|child_builder| {
            for body in bodies {
                (spawn_entities.f)(&spawn_entities, None, body, child_builder, 0);
            }
        });
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
