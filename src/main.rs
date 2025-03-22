use bevy::{color::palettes::css::WHITE, prelude::*};
use bevy_flycam::*;
use bevy_rapier3d::prelude::*;
use bevy_rl::*;
use bevy_stl::StlPlugin;
use serde::{Deserialize, Serialize};

use bevy_urdf::events::{ControlMotors, LoadRobot, RobotLoaded};
use bevy_urdf::events::{SensorsRead, SpawnRobot};
use bevy_urdf::plugin::UrdfPlugin;
use bevy_urdf::urdf_asset_loader::UrdfAsset;

use rand::Rng;

fn main() {
    App::new()
        .insert_resource(AIGymState::<Actions, EnvironmentState>::new(
            AIGymSettings {
                num_agents: 1,
                render_to_buffer: false,
                pause_interval: 0.1,
                ..default()
            },
        ))
        .add_plugins((
            DefaultPlugins,
            UrdfPlugin,
            StlPlugin,
            NoCameraPlayerPlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
            AIGymPlugin::<Actions, EnvironmentState>::default(),
        ))
        .init_state::<AppState>()
        .insert_resource(MovementSettings {
            speed: 1.0,
            ..default()
        })
        .insert_resource(UrdfRobotHandle(None))
        .insert_resource(LastSensortReading(None))
        .add_systems(Startup, setup)
        .add_systems(Update, start_simulation.run_if(in_state(AppState::Loading)))
        .add_systems(
            Update,
            (bevy_rl_pause_request, bevy_rl_control_request, read_sensors)
                .run_if(in_state(AppState::Simulation)),
        )
        .run();
}

// Bevy URDF stuff

#[derive(Resource)]
struct UrdfRobotHandle(Option<Handle<UrdfAsset>>);

#[derive(Resource)]
struct LastSensortReading(Option<EnvironmentState>);

// Bevy RL stuff

#[derive(Default, Deref, DerefMut, Clone, Deserialize)]
pub struct Actions(Vec<f64>);

#[derive(Default, Serialize, Clone)]
pub struct EnvironmentState {
    pub transforms: Vec<[f32; 7]>,
    pub joint_angles: Vec<f32>,
}

// bevy_rl systems

fn bevy_rl_pause_request(
    mut er_pause: EventReader<EventPause>,
    mut q_rapier_context_simulation: Query<(&mut RapierConfiguration)>,
    ai_gym_state: Res<AIGymState<Actions, EnvironmentState>>,
    last_sensors_readings: Res<LastSensortReading>,
    mut simulation_state: ResMut<NextState<SimulationState>>,
) {
    for _ in er_pause.read() {
        if let Some(state) = last_sensors_readings.0.clone() {
            for mut rapier_configuration in q_rapier_context_simulation.iter_mut() {
                if !rapier_configuration.physics_pipeline_active {
                    return;
                }
                rapier_configuration.physics_pipeline_active = false;
                simulation_state.set(SimulationState::PausedForControl);

                // Set bevy_rl gym state
                let mut ai_gym_state = ai_gym_state.lock().unwrap();
                ai_gym_state.set_env_state(state.clone());
            }
        }
    }
}

#[allow(unused_must_use)]
fn bevy_rl_control_request(
    mut er_control: EventReader<EventControl>,
    mut q_rapier_context_simulation: Query<(&mut RapierConfiguration)>,
    mut simulation_state: ResMut<NextState<SimulationState>>,
    mut ew_control_motors: EventWriter<ControlMotors>,
    robot_handle: Res<UrdfRobotHandle>,
) {
    for control in er_control.read() {
        println!("control");
        for mut rapier_configuration in q_rapier_context_simulation.iter_mut() {
            rapier_configuration.physics_pipeline_active = true;
            let raw_actions = control.0.clone();
            println!("simulation now active {}", raw_actions.len());

            if let Some(robot_handle) = robot_handle.0.clone() {
                for i in 0..raw_actions.len() {
                    if let Some(unparsed_action) = raw_actions[i].clone() {
                        let velocities: Vec<f32> = serde_json::from_str(&unparsed_action).unwrap();
                        println!("velocities: {:?}", velocities);
                        ew_control_motors.send(ControlMotors {
                            handle: robot_handle,
                            velocities: velocities,
                        });
                        break;
                    }
                }
            }

            simulation_state.set(SimulationState::Running);
        }
    }
}

fn start_simulation(
    mut commands: Commands,
    mut er_robot_loaded: EventReader<RobotLoaded>,
    mut ew_spawn_robot: EventWriter<SpawnRobot>,
    mut state: ResMut<NextState<AppState>>,
    mut simulation_state: ResMut<NextState<SimulationState>>,
) {
    for event in er_robot_loaded.read() {
        ew_spawn_robot.send(SpawnRobot {
            handle: event.handle.clone(),
            mesh_dir: event.mesh_dir.clone(),
        });
        state.set(AppState::Simulation);
        simulation_state.set(SimulationState::Running);
        commands.insert_resource(UrdfRobotHandle(Some(event.handle.clone())));
    }
}

fn read_sensors(mut commands: Commands, mut er_read_sensors: EventReader<SensorsRead>) {
    for event in er_read_sensors.read() {
        let mut transforms: Vec<[f32; 7]> = Vec::new();
        for transform in event.transforms.iter() {
            let t: [f32; 7] = [
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
                transform.rotation.x,
                transform.rotation.y,
                transform.rotation.z,
                transform.rotation.w,
            ];

            transforms.push(t);
        }

        let environment_state = EnvironmentState {
            transforms: transforms,
            joint_angles: event.joint_angles.clone(),
        };
        commands.insert_resource(LastSensortReading(Some(environment_state)));
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Loading,
    Simulation,
}

#[allow(deprecated)]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ew_load_robot: EventWriter<LoadRobot>,
    mut simulation_state: ResMut<NextState<SimulationState>>,
) {
    // Scene
    commands.insert_resource(AmbientLight {
        color: WHITE.into(),
        brightness: 300.0,
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        FlyCam,
    ));

    // ground
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(180., 1.8, 180.))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Collider::cuboid(90., 0.9, 90.),
        Transform::from_xyz(0.0, -2.5, 0.0),
        RigidBody::Fixed,
    ));

    simulation_state.set(SimulationState::Initializing);

    ew_load_robot.send(LoadRobot {
        urdf_path: "robots/flamingo_edu/urdf/Edu_v4.urdf".to_string(),
        mesh_dir: "assets/robots/flamingo_edu/urdf".to_string(),
    });
}
