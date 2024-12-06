use std::fs;

use anyhow::*;
use regex::Regex;
use roxmltree::*;

#[derive(Debug)]
pub struct Body {
    pub pos: (f32, f32, f32),
    pub name: String,
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

fn parse_body(element: &Node) -> Result<Body> {
    let pos_attr = element
        .attribute("pos")
        .ok_or_else(|| anyhow!("Missing 'pos' attribute in body element"))?;
    let pos = parse_pos(pos_attr)?;
    let name = element
        .attribute("name")
        .ok_or_else(|| anyhow!("Missing 'name' attribute in body element"))?;
    let body = Body {
        name: name.to_string(),
        pos,
    };

    Ok(body)
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

    for element in worldbody_element.children() {
        if element.tag_name().name() == "body" {
            let body = parse_body(&element)?;
            println!("Body: {:?}", body);
        } else {
            println!("Unknown element: {}", element.tag_name().name());
        }
    }

    Ok(())
}
