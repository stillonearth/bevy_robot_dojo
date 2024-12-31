#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![feature(let_chains)]

mod mujoco_parser;

use std::{cell::RefCell, rc::Rc};

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
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin,
        ))
        .insert_resource(WireframeConfig {
            // The global wireframe config enables drawing of wireframes on every mesh,
            // except those with `NoWireframe`. Meshes with `Wireframe` will always have a wireframe,
            // regardless of the global configuration.
            global: true,
            // Controls the default color of all wireframes. Used as the default color for global wireframes.
            // Can be changed per mesh using the `WireframeColor` component.
            default_color: WHITE.into(),
        })
        .add_plugins(WorldInspectorPlugin::new())
        .init_asset::<mujoco_parser::MuJoCoFile>()
        .init_asset_loader::<mujoco_parser::MuJoCoFileLoader>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (spawn_mujoco_model,).run_if(in_state(AppState::Loading)),
        )
        // .add_systems(Update, ().run_if(in_state(AppState::Simulation)))
        .init_state::<AppState>()
        .add_plugins(NoCameraPlayerPlugin)
        .insert_resource(MovementSettings {
            speed: 1.0,
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
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
        NoWireframe,
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4., 4.0).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        FlyCam,
    ));
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Loading,
    Simulation,
}

/// BodyTree restructures body list into a tree structure
/// All translations and quaternions are relative to the parent body
#[derive(Deref, DerefMut)]
pub struct BodyTree(pub Tree<Body>);

#[derive(Resource)]
struct MuJoCoFileHandle(Handle<MuJoCoFile>);

#[derive(Component)]
struct Shape;

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
        ) {
            let mesh = body.mesh();
            if mesh.is_none() {
                return;
            }
            let mesh = mesh.unwrap();
            let (x, z, y) = body.pos;
            let body_transform = Transform::from_xyz(x, y, z);
            let geom_transform: Transform = body.geom.transform();

            let mut binding: EntityCommands;
            {
                let mut materials = materials.borrow_mut();
                let mut meshes = meshes.borrow_mut();
                let mut images = images.borrow_mut();

                let body_name = body.name.clone().unwrap_or_default();

                binding = child_builder.spawn((
                    Name::new(format!("MuJoCo::body_{}", body_name.as_str())),
                    body_transform,
                ));

                if let Some(joint) = body.joint.clone() {
                    binding.insert(joint);
                }

                let debug_material = materials.add(StandardMaterial {
                    base_color_texture: Some(images.add(uv_debug_texture())),
                    ..default()
                });

                binding.with_children(|children| {
                    let mut cmd = children.spawn((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(debug_material.clone()),
                        geom_transform,
                        WireframeColor { color: LIME.into() },
                    ));

                    cmd.insert(Name::new(format!(
                        "MuJoCo::mesh_{}",
                        body.name.clone().unwrap_or_default().as_str()
                    )));
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
            );
        },
    };

    //
    // return;

    let mut commands = commands.borrow_mut();
    let bodies = mujoco_file.unwrap().0.clone();
    commands
        .spawn((Name::new("MuJoCo::world"), Transform::IDENTITY))
        .with_children(|child_builder| {
            for body in bodies {
                (spawn_entities.f)(&spawn_entities, body, child_builder, 0);
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
