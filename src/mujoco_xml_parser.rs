use avian3d::prelude::Collider;
use avian3d::prelude::MassPropertiesBundle;
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

#[derive(Debug, Serialize, Clone, Component)]
pub struct Joint {
    pub pos: (f32, f32, f32),
    pub axis: Option<(f32, f32, f32)>,
    pub range: Option<(f32, f32)>,
    pub name: Option<String>,
    pub joint_type: String,
    pub margin: Option<f32>,
}

#[derive(Debug, Serialize, Clone, Component)]
pub struct Body {
    pub pos: (f32, f32, f32),
    pub name: Option<String>,
    pub geom: Geom,
    pub children: Vec<Body>,
    pub joint: Option<Joint>,
}

impl Body {
    pub fn transform(&self) -> Transform {
        let (x, z, y) = self.pos;
        Transform::from_xyz(x, y, z)
    }
}

pub enum Shape {
    Sphere { object: Sphere },
    Capsule3d { object: Capsule3d },
}

#[derive(Debug, Serialize, Clone, Component)]
pub struct Geom {
    pub from: Option<(f32, f32, f32)>,
    pub to: Option<(f32, f32, f32)>,
    pub pos: Option<(f32, f32, f32)>,
    pub name: String,
    pub size: f32,
    pub geom_type: String,
}

impl Geom {
    pub fn mesh(&self) -> Mesh {
        let shape = self.shape();

        match shape {
            Shape::Sphere { object } => object.mesh().ico(5).unwrap(),
            Shape::Capsule3d { object } => object.mesh().build(),
        }
    }

    pub fn collider(&self) -> Collider {
        let shape = self.shape();
        match shape {
            Shape::Sphere { object } => Collider::sphere(object.radius),
            Shape::Capsule3d { object } => {
                Collider::capsule(object.radius, object.half_length * 2.0)
            }
        }
    }

    pub fn shape(&self) -> Shape {
        let size = self.size;

        match self.geom_type.as_str() {
            "sphere" => Shape::Sphere {
                object: Sphere { radius: size },
            },
            "capsule" => {
                if self.from.is_none() && self.to.is_none() {
                    Shape::Capsule3d {
                        object: Capsule3d::default(),
                    }
                } else {
                    let from = self.from.unwrap();
                    let to = self.to.unwrap();

                    let v1 = Vec3::new(from.0, from.1, from.2);
                    let v2 = Vec3::new(to.0, to.1, to.2);

                    let length = (v2 - v1).length();

                    Shape::Capsule3d {
                        object: Capsule3d {
                            half_length: length / 2.0,
                            radius: size,
                        },
                    }
                }
            }
            _ => todo!(),
        }
    }

    /// Return the body to be rendered
    pub fn rotation(&self) -> Quat {
        match self.geom_type.as_str() {
            "capsule" => {
                if let Some(from) = self.from
                    && let Some(to) = self.to
                {
                    let v1 = Vec3::new(from.0, from.2, from.1);
                    let v2 = Vec3::new(to.0, to.2, to.1);

                    let to = (v2 - v1).normalize();
                    let from = Vec3::new(0.0, 1.0, 0.0);
                    let rotation = Quat::from_rotation_arc(from, to);

                    return rotation;
                }

                Quat::IDENTITY
            }
            _ => Quat::IDENTITY,
        }
    }

    pub fn postion(&self) -> Vec3 {
        if let Some((x, z, y)) = self.pos {
            return Vec3::new(x, z, y);
        }

        match self.geom_type.as_str() {
            "capsule" => {
                if let Some(from) = self.from
                    && let Some(to) = self.to
                {
                    let v1 = Vec3::new(from.0, from.2, from.1);
                    let v2 = Vec3::new(to.0, to.2, to.1);

                    return (v2 - v1) / 2.0;
                }

                Vec3::ZERO
            }
            _ => Vec3::ZERO,
        }
    }

    pub fn mass_properties_bundle(&self) -> MassPropertiesBundle {
        let shape = self.shape();

        match shape {
            Shape::Sphere { object } => MassPropertiesBundle::from_shape(&object, 1.0),
            Shape::Capsule3d { object } => MassPropertiesBundle::from_shape(&object, 1.0),
        }
    }

    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.postion()).with_rotation(self.rotation())
    }
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
        name,
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

    Ok((from, to))
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

    Ok((x, y, z))
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

    Ok(((x1, y1, z1), (x2, y2, z2)))
}

fn parse_body(element: &roxmltree::Node) -> Result<Body> {
    let pos_attr = element
        .attribute("pos")
        .ok_or_else(|| anyhow!("Missing 'pos' attribute in body element"))?;
    let pos = parse_3_vec(pos_attr)?;
    let name = element.attribute("name").map(|s| s.to_string());
    let geom_node = element
        .children()
        .find(|n| n.tag_name().name() == "geom")
        .ok_or_else(|| anyhow!("Missing 'geom' node in body element"))?;

    let joint_node = element.children().find(|n| n.tag_name().name() == "joint");
    let joint = if joint_node.is_some() {
        Some(parse_joint(&joint_node.unwrap())?)
    } else {
        None
    };

    let body = Body {
        name,
        pos,
        geom: parse_geom(&geom_node)?,
        children: parse_parent_node(element)?,
        joint,
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
    let xml_document = roxmltree::Document::parse(document)?;

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
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content = std::str::from_utf8(&bytes).unwrap();
        let bodies = parse_mujoco_config(content).unwrap();

        Ok(MuJoCoFile(bodies))
    }
}
