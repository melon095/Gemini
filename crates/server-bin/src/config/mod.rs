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

pub trait GetProperty {
    fn get_property(&self, name: &str) -> Option<&Property>;

    fn get_property_of_string(&self, name: &str) -> Option<&str> {
        self.get_property(name).and_then(|p| match p.value {
            Value::String(s) => Some(s),
            _ => None,
        })
    }
    fn get_property_of_number(&self, name: &str) -> Option<u32> {
        self.get_property(name).and_then(|p| match p.value {
            Value::Number(n) => Some(n),
            _ => None,
        })
    }
}

pub fn read_and_parse_config(conf_str: &str) -> Result<Config> {
    let c = conf_str.trim();
    let c = config(c)?;

    Ok(c.1)
}
