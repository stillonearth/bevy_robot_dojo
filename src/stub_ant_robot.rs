use core::f32;

use avian3d::{math::*, prelude::*};
use bevy::prelude::*;

use crate::AppState;

pub fn spawn_stub_model(
    mut commands: Commands,
    mut app_state: ResMut<NextState<AppState>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    const SPHERE_RADIUS: f32 = 0.5;
    const CAPSULE_RADIUS: f32 = 0.1;
    const CAPSULE_LENGTH: f32 = 1.0;

    let sphere_mesh = meshes.add(Sphere::new(SPHERE_RADIUS));
    let capsule_mesh = meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH));
    let robot_material = materials.add(Color::srgb(0.8, 0.7, 0.6));

    // root body of ant
    let anchor = commands
        .spawn((
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(robot_material.clone()),
            // Transform::from_xyz(0.0, 5.0, 0.0),
            RigidBody::Dynamic,
            MassPropertiesBundle::from_shape(&Sphere::new(SPHERE_RADIUS), 1.0),
            Collider::sphere(SPHERE_RADIUS),
        ))
        .id();

    let leg_1_base = commands
        .spawn((
            Mesh3d(capsule_mesh.clone()),
            MeshMaterial3d(robot_material.clone()),
            Transform::from_xyz(1.5, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_z(f32::consts::FRAC_PI_2)),
            RigidBody::Dynamic,
            MassPropertiesBundle::from_shape(&Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH), 1.0),
            Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH),
        ))
        .id();

    // // Connect anchor and dynamic object
    commands.spawn(FixedJoint::new(anchor, leg_1_base).with_local_anchor_1(Vector::X * 1.5));

    app_state.set(AppState::Simulation);
}
