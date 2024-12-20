// use anyhow::*;
use bevy::asset::io::Reader;
use bevy::asset::AssetLoader;
use bevy::asset::LoadContext;
use bevy::prelude::*;
use regex::Regex;
use serde::Serialize;

use anyhow::anyhow;
use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Serialize)]
pub struct Body {
    pub pos: (f32, f32, f32),
    pub name: Option<String>,
    pub geom: Geom,
    pub children: Vec<Body>,
    pub joint: Option<Joint>,
}

#[derive(Debug, Serialize)]
pub struct Geom {
    pub from: Option<(f32, f32, f32)>,
    pub to: Option<(f32, f32, f32)>,
    pub pos: Option<(f32, f32, f32)>,
    pub name: String,
    pub size: f32,
    pub geom_type: String,
}

#[derive(Debug, Serialize)]
pub struct Joint {
    pub pos: (f32, f32, f32),
    pub axis: Option<(f32, f32, f32)>,
    pub range: Option<(f32, f32)>,
    pub name: Option<String>,
    pub joint_type: String,
    pub margin: Option<f32>,
}

fn parse_joint(element: &roxmltree::Node) -> Result<Joint> {
    let pos_attr = element
        .attribute("pos")
        .ok_or_else(|| anyhow!("Missing 'pos' attribute in body element"))?;
    let pos = parse_3_vec(pos_attr)?;
    let name = element.attribute("name").map(|s| s.to_string());
    let joint_type = element
        .attribute("type")
        .ok_or_else(|| anyhow!("Missing 'type' attribute in body element"))?
        .to_string();
    let range = if let Some(range_attr) = element.attribute("range") {
        Some(parse_range(range_attr)?)
    } else {
        None
    };
    let axis = if let Some(axis_attr) = element.attribute("axis") {
        Some(parse_3_vec(axis_attr)?)
    } else {
        None
    };

    let margin = if let Some(margin_attr) = element.attribute("margin") {
        let mrg: f32 = margin_attr.parse()?;
        Some(mrg)
    } else {
        None
    };

    let joint = Joint {
        name: name,
        pos,
        joint_type,
        range,
        axis,
        margin,
    };

    Ok(joint)
}

fn parse_range(repr: &str) -> Result<(f32, f32)> {
    let re =
        Regex::new(r"^(?P<from>[+-]?([0-9]*[.])?[0-9]+) (?P<to>[+-]?([0-9]*[.])?[0-9]+)$").unwrap();

    let coorinates = re
        .captures(repr)
        .ok_or_else(|| anyhow!("Can't parse attribute"))?;

    let from: f32 = coorinates["from"].parse()?;
    let to: f32 = coorinates["to"].parse()?;

    return Ok((from, to));
}

fn parse_3_vec(repr: &str) -> Result<(f32, f32, f32)> {
    let re =
        Regex::new(r"^(?P<x>[+-]?([0-9]*[.])?[0-9]+) (?P<y>[+-]?([0-9]*[.])?[0-9]+) (?P<z>[+-]?([0-9]*[.])?[0-9]+)$")
            .unwrap();

    let coorinates = re
        .captures(repr)
        .ok_or_else(|| anyhow!("Can't parse attribute"))?;

    let x: f32 = coorinates["x"].parse()?;
    let y: f32 = coorinates["y"].parse()?;
    let z: f32 = coorinates["z"].parse()?;

    return Ok((x, y, z));
}

fn parse_fromto(repr: &str) -> Result<((f32, f32, f32), (f32, f32, f32))> {
    let re =
        Regex::new(r"^(?<x1>-?\d+\.\d+)\s+(?<y1>-?\d+\.\d+)\s+(?<z1>-?\d+\.\d+)\s+(?<x2>-?\d+\.\d+)\s+(?<y2>-?\d+\.\d+)\s+(?<z2>-?\d+\.\d+)$")
            .unwrap();

    let coorinates = re
        .captures(repr)
        .ok_or_else(|| anyhow!("Can't parse attribute"))?;

    let x1: f32 = coorinates["x1"].parse()?;
    let y1: f32 = coorinates["y1"].parse()?;
    let z1: f32 = coorinates["z1"].parse()?;

    let x2: f32 = coorinates["x2"].parse()?;
    let y2: f32 = coorinates["y2"].parse()?;
    let z2: f32 = coorinates["z2"].parse()?;

    return Ok(((x1, y1, z1), (x2, y2, z2)));
}

fn parse_body(element: &roxmltree::Node) -> Result<Body> {
    let pos_attr = element
        .attribute("pos")
        .ok_or_else(|| anyhow!("Missing 'pos' attribute in body element"))?;
    let pos = parse_3_vec(pos_attr)?;
    let name = element.attribute("name").map(|s| s.to_string());
    let geom_node = element
        .descendants()
        .find(|n| n.tag_name().name() == "geom")
        .ok_or_else(|| anyhow!("Missing 'geom' node in body element"))?;

    let joint_node = element
        .descendants()
        .find(|n| n.tag_name().name() == "joint")
        .ok_or_else(|| anyhow!("Missing 'joint' node in body element"))?;
    let joint = parse_joint(&joint_node)?;

    let body = Body {
        name,
        pos,
        geom: parse_geom(&geom_node)?,
        children: parse_parent_node(element)?,
        joint: Some(joint),
    };

    Ok(body)
}

fn parse_geom(element: &roxmltree::Node) -> Result<Geom> {
    let mut from: Option<(f32, f32, f32)> = None;
    let mut to: Option<(f32, f32, f32)> = None;
    let mut pos: Option<(f32, f32, f32)> = None;

    if let Some(fromto_attr) = element.attribute("fromto") {
        let (from_, to_) = parse_fromto(fromto_attr)?;
        from = Some(from_);
        to = Some(to_);
    }

    if let Some(pos_attr) = element.attribute("pos") {
        let pos_ = parse_3_vec(pos_attr)?;
        pos = Some(pos_);
    }

    let name = element
        .attribute("name")
        .ok_or_else(|| anyhow!("Missing 'name' attribute in body element"))?
        .to_string();
    let geom_type = element
        .attribute("type")
        .ok_or_else(|| anyhow!("Missing 'type' attribute in body element"))?
        .to_string();
    let size = element
        .attribute("size")
        .ok_or_else(|| anyhow!("Missing 'size' attribute in body element"))?;

    let body = Geom {
        name,
        from,
        pos,
        to,
        geom_type,
        size: size.parse()?,
    };

    Ok(body)
}

pub fn parse_parent_node(node: &roxmltree::Node) -> Result<Vec<Body>> {
    let mut bodies: Vec<Body> = vec![];

    for element in node.children() {
        if element.tag_name().name() == "body" {
            let body = parse_body(&element)?;
            bodies.push(body);
        }
    }

    Ok(bodies)
}

pub fn parse_mujoco_config(document: &str) -> Result<Vec<Body>> {
    let xml_document = roxmltree::Document::parse(&document)?;

    let worldbody_element = xml_document
        .descendants()
        .find(|n| n.tag_name().name() == "worldbody")
        .unwrap();

    let bodies = parse_parent_node(&worldbody_element)?;

    Ok(bodies)
}

// Custom Loader

#[derive(Default)]
pub struct MuJoCoFileLoader;

#[derive(Asset, TypePath, Debug, Deref)]
pub struct MuJoCoFile(pub Vec<Body>);

/// Possible errors that can be produced by [`RpyAssetLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MuJoCoFileLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load file: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for MuJoCoFileLoader {
    type Asset = MuJoCoFile;
    type Settings = ();
    type Error = MuJoCoFileLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        info!("Loading XML...");
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content = std::str::from_utf8(&bytes).unwrap();
        let bodies = parse_mujoco_config(content).unwrap();

        println!("loaded asset, {:?}", bodies);

        Ok(MuJoCoFile(bodies))
    }
}
