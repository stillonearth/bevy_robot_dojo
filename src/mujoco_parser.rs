use std::fs;

use anyhow::*;
use regex::Regex;
use roxmltree::*;

#[derive(Debug)]
pub struct Body {
    pub pos: (f32, f32, f32),
    pub name: String,
    pub geom: Geom,
    pub children: Vec<Body>,
}

#[derive(Debug)]
pub struct Geom {
    pub from: Option<(f32, f32, f32)>,
    pub to: Option<(f32, f32, f32)>,
    pub pos: Option<(f32, f32, f32)>,
    pub name: String,
    pub size: f32,
    pub geom_type: String,
}

fn parse_pos(repr: &str) -> Result<(f32, f32, f32)> {
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
        Regex::new(r"^(?P<x1>[+-]?([0-9]*[.])?[0-9]+) (?P<y1>[+-]?([0-9]*[.])?[0-9]+) (?P<z1>[+-]?([0-9]*[.])?[0-9]+ (?P<x2>[+-]?([0-9]*[.])?[0-9]+) (?P<y2>[+-]?([0-9]*[.])?[0-9]+) (?P<z2>[+-]?([0-9]*[.])?[0-9]+))$")
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

fn parse_body(element: &Node) -> Result<Body> {
    let pos_attr = element
        .attribute("pos")
        .ok_or_else(|| anyhow!("Missing 'pos' attribute in body element"))?;
    let pos = parse_pos(pos_attr)?;
    let name = element
        .attribute("name")
        .ok_or_else(|| anyhow!("Missing 'name' attribute in body element"))?;
    let geom_node = element
        .descendants()
        .find(|n| n.tag_name().name() == "geom")
        .ok_or_else(|| anyhow!("Missing 'geom' node in body element"))?;

    let body = Body {
        name: name.to_string(),
        pos,
        geom: parse_geom(&geom_node)?,
        children: vec![], //parse_parent_node(element)?,
    };

    Ok(body)
}

fn parse_geom(element: &Node) -> Result<Geom> {
    let mut from: Option<(f32, f32, f32)> = None;
    let mut to: Option<(f32, f32, f32)> = None;
    let mut pos: Option<(f32, f32, f32)> = None;

    if let Some(fromto_attr) = element.attribute("fromto") {
        let (from_, to_) = parse_fromto(fromto_attr)?;
        from = Some(from_);
        to = Some(to_);
    }

    if let Some(pos_attr) = element.attribute("pos") {
        let pos_ = parse_pos(pos_attr)?;
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

pub fn parse_parent_node(node: &Node) -> Result<Vec<Body>> {
    let mut bodies: Vec<Body> = vec![];

    for element in node.children() {
        if element.tag_name().name() == "body" {
            let body = parse_body(&element)?;
            bodies.push(body);
        }
    }

    Ok(bodies)
}

pub fn parse_mujoco_config(filename: &str) -> Result<()> {
    // load filename file to String
    let document = fs::read_to_string(filename)?;
    let xml_document = roxmltree::Document::parse(&document)?;

    // 1. Parse Worldbody
    let worldbody_element = xml_document
        .descendants()
        .find(|n| n.tag_name().name() == "worldbody")
        .unwrap();

    let bodies = parse_parent_node(&worldbody_element)?;
    for body in bodies.iter() {
        println!("body {:?}", body);
    }

    Ok(())
}
