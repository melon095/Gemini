use crate::config::{error::Error, parser::config};
use std::collections::HashMap;
use std::fmt::Display;

pub mod error;
pub mod parser;

pub type Properties<'a, 'b> = HashMap<&'a str, Property<'b>>;
pub type Result<'a, T> = std::result::Result<T, Error<'a>>;

#[derive(Debug, Eq, PartialEq)]
pub struct Tag<'a>(pub &'a str);

#[derive(Debug, Eq, PartialEq)]
struct Block<'a> {
    pub tag: Tag<'a>,
    pub properties: Properties<'a, 'a>,
    pub children: Vec<Block<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Server<'a> {
    pub properties: Properties<'a, 'a>,
    pub vhosts: Vec<VHost<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VHost<'a> {
    pub vhost: Tag<'a>,
    pub properties: Properties<'a, 'a>,
    pub routes: Vec<Route<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Route<'a> {
    pub path: Tag<'a>,
    pub properties: Properties<'a, 'a>,
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
    pub server: Server<'a>,
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

impl GetProperty for Server<'_> {
    fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.get(name)
    }
}

impl GetProperty for VHost<'_> {
    fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.get(name)
    }
}

impl GetProperty for Route<'_> {
    fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.get(name)
    }
}

impl<'a> GetProperty for Config<'a> {
    fn get_property(&self, name: &str) -> Option<&Property> {
        self.server.get_property(name)
    }
}

impl<'a> From<&'a str> for Tag<'a> {
    fn from(s: &'a str) -> Self {
        Tag(s)
    }
}

impl<'a> TryFrom<&Property<'a>> for Tag<'a> {
    type Error = Error<'a>;

    fn try_from(p: &Property<'a>) -> Result<'a, Tag<'a>> {
        match p.value {
            Value::String(s) => Ok(Tag(s)),
            _ => Err(Error::InvalidBlockTag(p.name.parse().unwrap())),
        }
    }
}

impl Display for Tag<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> TryFrom<Block<'a>> for Server<'a> {
    type Error = Error<'a>;

    fn try_from(block: Block<'a>) -> Result<'a, Server<'a>> {
        let properties = block.properties;
        let vhosts = block
            .children
            .into_iter()
            .filter(|b| b.tag.0 == "vhost")
            .map(VHost::try_from)
            .collect::<Result<_>>()?;

        Ok(Server { properties, vhosts })
    }
}

impl<'a> TryFrom<Block<'a>> for VHost<'a> {
    type Error = Error<'a>;

    fn try_from(block: Block<'a>) -> Result<'a, VHost<'a>> {
        if block.tag.0 != "vhost" {
            return Err(Error::InvalidBlockTag(format!(
                "Expected 'vhost', got '{}'",
                block.tag.0
            )));
        }

        let vhost = block
            .properties
            .get("hostname")
            .ok_or_else(|| Error::UnableToMaterializeStructure("Missing 'hostname' property"))?;

        let vhost = Tag::try_from(vhost)?;

        let properties = block.properties;
        let routes = block
            .children
            .into_iter()
            .filter(|b| b.tag.0 == "route")
            .map(Route::try_from)
            .collect::<Result<_>>()?;

        Ok(VHost {
            vhost,
            properties,
            routes,
        })
    }
}

impl<'a> TryFrom<Block<'a>> for Route<'a> {
    type Error = Error<'a>;

    fn try_from(block: Block<'a>) -> Result<'a, Route<'a>> {
        if block.tag.0 != "route" {
            return Err(Error::InvalidBlockTag(format!(
                "Expected 'route', got '{}'",
                block.tag.0
            )));
        }

        let path = block
            .properties
            .get("path")
            .ok_or_else(|| Error::UnableToMaterializeStructure("missing 'path'"))?;

        let path = Tag::try_from(path)?;

        let properties = block.properties;

        Ok(Route { path, properties })
    }
}

pub fn read_and_parse_config(conf_str: &str) -> Result<Config> {
    let c = config(conf_str)?;

    Ok(c.1)
}
