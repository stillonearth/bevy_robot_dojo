#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;

mod mujoco_parser;

fn main() {
    // App::new()
    //     .add_plugins(DefaultPlugins)
    //     .add_systems(Startup, setup)
    //     .run();

    mujoco_parser::parse_mujoco_config("assets/ant.xml");
}

// fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {}
