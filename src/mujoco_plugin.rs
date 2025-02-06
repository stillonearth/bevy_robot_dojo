use std::{cell::RefCell, rc::Rc};

use avian3d::prelude::{Joint, *};
use bevy::{color::palettes::css::*, pbr::wireframe::WireframeColor, prelude::*};

use crate::mujoco_xml_parser;
use crate::mujoco_xml_parser::{Body, Geom, MuJoCoFile};

use crate::AppState;

#[derive(Component)]
pub struct MuJoCoRoot;

#[derive(Resource)]
pub struct MuJoCoFileHandle(pub Handle<MuJoCoFile>);

#[derive(Component)]
pub struct GeomWrapper;

pub fn spawn_mujoco_model(
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
            if depth == 3 {
                return;
            }

            let mut binding_1: EntityCommands;
            {
                // let mut commands = commands.borrow_mut();
                let _materials = materials.borrow_mut();
                let mut meshes = meshes.borrow_mut();
                let _images = images.borrow_mut();

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
                    let joint = crate::mujoco_xml_parser::Joint {
                        joint_type: "none".to_string(),
                        pos: (0.0, 0.0, 0.0),
                        axis: None,
                        range: None,
                        name: None,
                        margin: None,
                    };
                    binding_1.insert(joint);
                }

                binding_1.with_children(|children| {
                    let mut binding_2: EntityCommands<'_> = children.spawn((
                        // RigidBody::Dynamic,
                        GeomWrapper {},
                        Name::new(format!("MuJoCo::geom_wrapper_{}", body_name.as_str())),
                    ));

                    if depth == 1 {
                        binding_2.insert(RigidBody::Dynamic);
                    } else {
                        binding_2.insert(RigidBody::Kinematic);
                    }

                    binding_2.with_children(|children| {
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
                                RigidBody::Dynamic,
                                body.geom.mass_properties_bundle(),
                                // body.geom.collider(),
                            ));
                        } else {
                            cmd.insert((
                                // RigidBody::Static,
                                body.geom.mass_properties_bundle(),
                                // body.geom.collider(),
                            ));
                        }

                        if depth == 0 {
                            cmd.insert((body.geom.collider(),));
                        }
                    });
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

pub fn spawn_mujoco_joints(
    mut commands: Commands,
    // q_mujoco_root: Query<(Entity, &MuJoCoRoot)>,
    q_joints: Query<(Entity, &Parent, &mujoco_xml_parser::Joint)>,
    q_geoms: Query<(Entity, &Parent, &Geom)>,
    q_geom_wrappers: Query<(Entity, &Parent, &GeomWrapper)>,
) {
    // iterate over joints and inser avian joint
    for (entity, joint_parent, joint) in q_joints.iter() {
        // link parent geom to child geom wrapper to include avian rotations
        if joint.joint_type == "none" {
            // find upper mesh
            let parent_geom_wrapper = q_geom_wrappers
                .iter()
                .find(|(_, p2, _)| p2.get() == joint_parent.get());

            if parent_geom_wrapper.is_none() {
                continue;
            }

            let (parent_geom_wrapper, _, _) = parent_geom_wrapper.unwrap();
            let parent_geom = q_geoms
                .iter()
                .find(|(_, p, _)| p.get() == parent_geom_wrapper);
            let (parent_geom, _, _) = parent_geom.unwrap();

            let child_geom_wrapper = q_geom_wrappers.iter().find(|(_, p1, _)| p1.get() == entity);
            if child_geom_wrapper.is_none() {
                continue;
            }
            let (child_geom_wrapper, _, _) = child_geom_wrapper.unwrap();

            commands.spawn((
                FixedJoint::new(child_geom_wrapper, parent_geom),
                Name::new("Fixed Joint"),
            ));
            // link parent geom to child geom to include avian rotations
        } else if joint.joint_type == "hinge" {
            let parent_geom_wrapper = q_geom_wrappers
                .iter()
                .find(|(_, p2, _)| p2.get() == joint_parent.get());

            if parent_geom_wrapper.is_none() {
                continue;
            }

            let (parent_geom_wrapper, _, _) = parent_geom_wrapper.unwrap();
            let parent_geom = q_geoms
                .iter()
                .find(|(_, p, _)| p.get() == parent_geom_wrapper);
            let (parent_geom, _, _) = parent_geom.unwrap();

            let child_geom_wrapper = q_geom_wrappers.iter().find(|(_, p1, _)| p1.get() == entity);
            if child_geom_wrapper.is_none() {
                continue;
            }
            let (child_geom_wrapper, _, _) = child_geom_wrapper.unwrap();
            let child_geom = q_geoms
                .iter()
                .find(|(_, p, _)| p.get() == child_geom_wrapper);
            let (_child_geom, _, _) = child_geom.unwrap();

            commands.spawn((FixedJoint::new(parent_geom, child_geom_wrapper),));
        }
    }
}
