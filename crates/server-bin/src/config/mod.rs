use crate::config::{error::Error, parser::config};
use std::collections::HashMap;

pub mod error;
pub mod parser;

pub type Properties<'a, 'b> = HashMap<&'a str, Property<'b>>;
pub type Result<'a, T> = std::result::Result<T, Error<'a>>;

#[derive(Debug, Eq, PartialEq)]
pub struct Tag<'a>(&'a str);

#[derive(Debug, Eq, PartialEq)]
pub struct Block<'a> {
    pub tag: Tag<'a>,
    pub properties: Properties<'a, 'a>,
    pub children: Vec<Block<'a>>,
    pub variant: BlockVariant,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BlockVariant {
    Server,
    Vhost,
    Route,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Number(u32),
}

#[derive(Debug, Eq, PartialEq)]
pub struct Property<'a> {
    name: &'a str,
    value: Value<'a>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Config<'a> {
    pub server: Block<'a>,
}

pub trait GetProperty {
    fn get_property(&self, name: &str) -> Option<&Property>;

    fn get_property_string(&self, name: &str) -> Option<&str> {
        self.get_property(name).and_then(|p| match p.value {
            Value::String(s) => Some(s),
            _ => None,
        })
    }
    fn get_property_number(&self, name: &str) -> Option<u32> {
        self.get_property(name).and_then(|p| match p.value {
            Value::Number(n) => Some(n),
            _ => None,
        })
    }
}

impl<'a> GetProperty for Block<'a> {
    fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.get(name)
    }
}

impl<'a> GetProperty for Config<'a> {
    fn get_property(&self, name: &str) -> Option<&Property> {
        self.server.get_property(name)
    }
}

pub trait GetBlocks {
    fn get_blocks(&self, tag: &str) -> Vec<&Block>;
}

impl<'a> GetBlocks for Block<'a> {
    fn get_blocks(&self, tag: &str) -> Vec<&Block> {
        let mut blocks = Vec::new();

        for child in &self.children {
            if child.tag.0 == tag {
                blocks.push(child);
            }
        }

        blocks
    }
}

impl<'a> GetBlocks for Config<'a> {
    fn get_blocks(&self, tag: &str) -> Vec<&Block> {
        self.server.get_blocks(tag)
    }
}

pub fn read_and_parse_config(conf_str: &str) -> Result<Config> {
    let c = config(conf_str)?;

    Ok(c.1)
}
