use bevy::input::common_conditions::input_toggle_active;
use bevy::transform;
use bevy::{color::palettes::css::WHITE, prelude::*};
use bevy_flycam::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;
use bevy_rl::*;
use bevy_stl::StlPlugin;
use serde::{Deserialize, Serialize};

use bevy_urdf::events::{ControlMotors, LoadRobot, RobotLoaded, URDFRobot};
use bevy_urdf::events::{SensorsRead, SpawnRobot};
use bevy_urdf::plugin::UrdfPlugin;
use bevy_urdf::urdf_asset_loader::UrdfAsset;

// const stuff

const URDF_PATH: &str = "robots/flamingo_edu/urdf/Edu_v4.urdf";
const MESH_DIR: &str = "assets/robots/flamingo_edu/urdf";

// bevy_urdf resources

#[derive(Resource)]
struct UrdfRobotHandle(Option<Handle<UrdfAsset>>);

#[derive(Resource)]
struct LastSensorReading(Option<EnvironmentState>);

// bevy_rl state / actions

#[derive(Default, Deref, DerefMut, Clone, Deserialize)]
pub struct Actions(Vec<f64>);

#[derive(Default, Serialize, Clone)]
pub struct EnvironmentState {
    pub transforms: Vec<[f32; 7]>,
    pub joint_angles: Vec<f32>,
}

#[derive(Resource)]
pub struct SimulationData {
    previous_translation: Vec3,
}

// bevy_rl event handlers

fn handle_reset_event(
    mut commands: Commands,
    mut er_reset: EventReader<EventReset>,
    q_urdf_robots: Query<(Entity, &URDFRobot)>,
    robot_handle: Res<UrdfRobotHandle>,
    mut ew_spawn_robot: EventWriter<SpawnRobot>,
    ai_gym_state: Res<AIGymState<Actions, EnvironmentState>>,
) {
    for _ in er_reset.read() {
        if let Some(robot_handle) = robot_handle.0.clone() {
            for (entity, _) in q_urdf_robots.iter() {
                commands.entity(entity).despawn_recursive();
            }

            ew_spawn_robot.send(SpawnRobot {
                handle: robot_handle.clone(),
                mesh_dir: String::from(MESH_DIR).replace("assets/", ""),
                parent_entity: None,
            });

            let ai_gym_state = ai_gym_state.lock().unwrap();
            ai_gym_state.send_reset_result(true);
        }
    }
}

fn handle_pause_event(
    mut er_pause: EventReader<EventPause>,
    mut q_rapier_context_simulation: Query<(&mut RapierConfiguration)>,
    ai_gym_state: Res<AIGymState<Actions, EnvironmentState>>,
    last_sensors_readings: Res<LastSensorReading>,
    mut simulation_state: ResMut<NextState<SimulationState>>,
    q_urdf_robots: Query<(Entity, &Transform, &URDFRobot)>,
    mut simulation_data: ResMut<SimulationData>,
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

                for (i, (_, transform, _)) in q_urdf_robots.iter().enumerate() {
                    let previous_translation = simulation_data.previous_translation;
                    let current_translation = transform.translation;

                    let distance = current_translation.distance(previous_translation);
                    ai_gym_state.set_reward(i, distance);
                    simulation_data.previous_translation = current_translation.clone();
                }
            }
        }
    }
}

#[allow(unused_must_use)]
fn handle_control_request(
    mut er_control: EventReader<EventControl>,
    mut q_rapier_context_simulation: Query<(&mut RapierConfiguration)>,
    mut simulation_state: ResMut<NextState<SimulationState>>,
    mut ew_control_motors: EventWriter<ControlMotors>,
    robot_handle: Res<UrdfRobotHandle>,
) {
    for control in er_control.read() {
        for mut rapier_configuration in q_rapier_context_simulation.iter_mut() {
            rapier_configuration.physics_pipeline_active = true;
            let raw_actions = control.0.clone();

            if let Some(robot_handle) = robot_handle.0.clone() {
                for i in 0..raw_actions.len() {
                    if let Some(unparsed_action) = raw_actions[i].clone() {
                        let velocities: Vec<f32> = serde_json::from_str(&unparsed_action).unwrap();
                        // println!("velocities: {:?}", velocities);
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

fn handle_sensors_read(mut commands: Commands, mut er_read_sensors: EventReader<SensorsRead>) {
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
        commands.insert_resource(LastSensorReading(Some(environment_state)));
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
            parent_entity: None,
        });
        state.set(AppState::Simulation);
        simulation_state.set(SimulationState::Running);
        commands.insert_resource(UrdfRobotHandle(Some(event.handle.clone())));
    }
}

#[allow(deprecated)]
fn setup_scene(
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
            transform: Transform::from_xyz(0.0, 8.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        // FlyCam,
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
        urdf_path: URDF_PATH.into(),
        mesh_dir: MESH_DIR.into(),
        interaction_groups: None,
        marker: None,
    });
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Loading,
    Simulation,
}

fn main() {
    App::new()
        .insert_resource(AIGymState::<Actions, EnvironmentState>::new(
            AIGymSettings {
                num_agents: 1,
                render_to_buffer: false,
                pause_interval: 0.05,
                ..default()
            },
        ))
        .add_plugins((
            DefaultPlugins,
            UrdfPlugin,
            StlPlugin,
            // NoCameraPlayerPlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
            AIGymPlugin::<Actions, EnvironmentState>::default(),
            WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::Escape)),
        ))
        .init_state::<AppState>()
        .insert_resource(MovementSettings {
            speed: 1.0,
            ..default()
        })
        .insert_resource(UrdfRobotHandle(None))
        .insert_resource(LastSensorReading(None))
        .insert_resource(SimulationData {
            previous_translation: Vec3::ZERO,
        })
        // .insert_resource(RapierFixedTimeStep {
        //     fixed_time_step: 0.005, // 200x faster. Adjust this!
        // })
        .add_systems(Startup, setup_scene)
        .add_systems(Update, start_simulation.run_if(in_state(AppState::Loading)))
        .add_systems(
            Update,
            (
                handle_pause_event,
                handle_control_request,
                handle_sensors_read,
                handle_reset_event,
            )
                .run_if(in_state(AppState::Simulation)),
        )
        .run();
}
