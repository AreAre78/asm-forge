use std::{collections::HashMap, fs};

use serde::Deserialize;
use smart_default::SmartDefault;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub meta: ConfigMeta,
    #[serde(default)]
    pub architecture: ConfigArchitecture,
    #[serde(rename = "const")]
    #[serde(default)]
    pub const_: ConfigConst,
    #[serde(default)]
    pub syntax: ConfigSyntax,

    #[serde(default)]
    pub instruction: Vec<ConfigInstruction>,

    #[serde(flatten)]
    pub data: HashMap<String, ConfigData>,
}

#[derive(Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct ConfigMeta {
    pub architecture: String,
    pub variant: String,
    pub description: String,
}

#[derive(Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct ConfigArchitecture {
    #[default = 8]
    pub word_size: u32,
    #[default(Endianness::Big)]
    pub endian: Endianness,
    #[default = 1]
    pub align: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum Endianness {
    #[serde(rename = "big")]
    Big,

    #[serde(rename = "little")]
    Little,
}

#[derive(Debug, Deserialize, SmartDefault)]
pub struct ConfigConst {
    #[serde(flatten)]
    pub consts: HashMap<String, ConfigValue>,
}

#[derive(Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct ConfigSyntax {
    pub comment: Vec<String>,
    pub is_case_sensitive: bool,
}

#[derive(Debug, Deserialize)]
pub struct ConfigInstruction {
    pub pattern: String,
    pub encoding: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ConfigData {
    Simple {
        #[serde(default)]
        description: String,
        width: Option<u32>,
        items: HashMap<String, ConfigValue>,
    },
    Complex {
        #[serde(default)]
        description: String,
        width: Option<u32>,
        #[serde(default)]
        items: Vec<ConfigComplexData>,
    },
}

#[derive(Debug, Deserialize)]
pub struct ConfigComplexData {
    pub pattern: String,
    pub data: HashMap<String, ConfigValue>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Number(u32),
}

pub fn load(input: &str) -> Config {
    let input = fs::read_to_string("testing/example.toml").unwrap();

    Config::deserialize(toml::Deserializer::new(&input)).unwrap()
}
