#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod mujoco_parser;

use bevy::prelude::*;
use mujoco_parser::MuJoCoFile;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // MuJoCo Assetg Loader
        .init_asset::<mujoco_parser::MuJoCoFile>()
        .init_asset_loader::<mujoco_parser::MuJoCoFileLoader>()
        // Init and load asset
        .add_systems(Startup, setup)
        // .init_asset_loader::<rpy_asset_loader::BlobAssetLoader>()
        // .init_asset::<rpy_asset_loader::Blob>()
        .run();
}

fn setup(asset_server: Res<AssetServer>) {
    let _mujoco_handle: Handle<MuJoCoFile> = asset_server.load("ant.xml");
}
