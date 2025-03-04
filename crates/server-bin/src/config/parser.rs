use crate::config::{
    error::Error, Block, Config, Properties, Property, Result, Server, Tag, Value,
};
use std::collections::HashMap;

const SEMICOLON: char = ';';

fn take_inclusive(c: char) -> impl Fn(&str) -> Result<(&str, bool)> {
    move |i| {
        let len = i
            .chars()
            .position(|ch| ch == c)
            .map_or_else(|| i.len(), |pos| pos + 1);

        Ok((i[len..].trim_start(), len > 0))
    }
}

/// Combines two parsing functions into a single function that tries the first parser,
/// and if it fails, tries the second parser.
fn alt<'a, F, G, O>(f: F, g: G) -> impl Fn(&'a str) -> Result<'a, (&'a str, O)>
where
    F: Fn(&'a str) -> Result<'a, (&'a str, O)>,
    G: Fn(&'a str) -> Result<'a, (&'a str, O)>,
{
    move |i| {
        let res = f(i);
        if res.is_ok() { res } else { g(i) }
    }
}

/// ident = { alpha | "_" }
fn ident(i: &str) -> Result<(&str, &str)> {
    let mut len = 0;
    for c in i.chars() {
        if c.is_alphabetic() || c.eq(&'_') {
            len += 1;
        } else {
            break;
        }
    }

    match len {
        0 => Err(Error::ExpectedIdentifier(i.trim())),
        _ => Ok((i[len..].trim_start(), i[..len].trim())),
    }
}

fn string(i: &str) -> Result<(&str, Value)> {
    if !i.starts_with('"') {
        return Err(Error::StringExpectedStartingQuote(i));
    }

    let chars = i.chars();
    let mut string_len = 0;
    let mut found_end = false;

    // Skip the first quote
    for (idx, c) in chars.enumerate().skip(1) {
        string_len = idx;
        if c == '"' {
            found_end = true;
            string_len += 1;
            break;
        }
    }

    if !found_end {
        return Err(Error::StringExpectedEndingQuote(i));
    }

    Ok((
        &i[string_len..].trim_start(),
        Value::String(&i[1..string_len - 1]),
    ))
}

fn number(i: &str) -> Result<(&str, Value)> {
    let chars = i.chars();
    let mut number_len = 0;
    for c in chars {
        if c.is_numeric() {
            number_len += 1;
        } else {
            break;
        }
    }

    if number_len == 0 {
        return Err(Error::InvalidNumber(i.trim()));
    }

    let number_str = &i[..number_len];
    let number = number_str
        .parse()
        .map_err(|_| Error::InvalidNumber(number_str))?;

    Ok((&i[number_len..], Value::Number(number)))
}

fn property_with_name<'a>(i: &'a str, name: &'a str) -> Result<'a, (&'a str, Property<'a>)> {
    let (i, value) = alt(string, number)(i)?;
    // FIXME This semicolon checking is terrible and doesn't work.
    let (i, _) = take_inclusive(SEMICOLON)(i)?;

    Ok((i, Property { name, value }))
}

fn block_with_tag<'a>(i: &'a str, tag: &'a str) -> Result<'a, (&'a str, Block<'a>)> {
    // {
    let (i, _) = take_inclusive('{')(i)?;
    // property(ies) and block(s)
    let (i, properties, blocks) = properties_and_blocks(i)?;
    // }
    let (i, _) = take_inclusive('}')(i)?;

    Ok((
        i,
        Block {
            tag: Tag(tag),
            properties,
            children: blocks,
        },
    ))
}

fn properties_and_blocks(i: &str) -> Result<(&str, Properties, Vec<Block>)> {
    let mut props = HashMap::new();
    let mut blocks = Vec::new();
    let mut i = i;
    loop {
        let (i_, name) = ident(i)?;
        let mut i_ = i_.trim_start();
        if i_.starts_with('{') {
            let (i, block) = block(i)?;
            blocks.push(block);
            i_ = i;
        } else {
            let (i, property) = property_with_name(i_, name)?;
            props.insert(property.name, property);
            i_ = i;
        }

        if i_.is_empty() || i_.starts_with('}') {
            break;
        }

        i = i_;
    }

    Ok((i, props, blocks))
}

fn block(i: &str) -> Result<(&str, Block)> {
    // IDENT
    let (i, tag) = ident(i)?;
    block_with_tag(i, tag)
}

fn server(i: &str) -> Result<(&str, Server)> {
    let (i, block) = block(i)?;
    Ok((i, Server::try_from(block)?))
}

pub(super) fn config(i: &str) -> Result<(&str, Config)> {
    let i_ = i.trim_start();
    let (_, server) = server(i_)?;

    Ok((i, Config { server }))
}

#[cfg(test)]
mod tests {
    use crate::config::error::Error::*;
    use crate::config::parser::Value;
    use crate::config::read_and_parse_config;

    #[test]
    fn test_file() {
        let input = r#"
server
{
    port 1965;

    vhost
    {
        hostname  "localhost";
        tls_cert  "cert.pem";
        tls_key   "key.key";

        route
        {
            path         "/index";
            respond_body "=> Hello, World!";
        }
    }
}
    "#;

        let config = read_and_parse_config(input);
        assert!(config.is_ok());
    }

    #[test]
    fn test_string() {
        let cases = vec![
            ("hello", Err(StringExpectedStartingQuote("hello"))),
            (r#"hello""#, Err(StringExpectedStartingQuote("hello\""))),
            (r#""hello"world"#, Ok(("world", Value::String("hello")))),
            (
                r#""unterminated"#,
                Err(StringExpectedEndingQuote("\"unterminated")),
            ),
            ("\"", Err(StringExpectedEndingQuote("\""))),
            ("''", Err(StringExpectedStartingQuote("''"))),
            (r#""""#, Ok(("", Value::String("")))),
            ("", Err(StringExpectedStartingQuote(""))),
            (" ", Err(StringExpectedStartingQuote(" "))),
            (r#"42"#, Err(StringExpectedStartingQuote("42"))),
        ];

        for (input, expected) in cases {
            assert_eq!(super::string(input), expected);
        }
    }

    #[test]
    fn test_number() {
        let cases = vec![
            ("hello", Err(InvalidNumber("hello"))),
            ("42", Ok(("", Value::Number(42)))),
            ("42 ", Ok((" ", Value::Number(42)))),
            ("42hello", Ok(("hello", Value::Number(42)))),
            ("42.0", Ok((".0", Value::Number(42)))),
            ("42.0 ", Ok((".0 ", Value::Number(42)))),
            ("42.0hello", Ok((".0hello", Value::Number(42)))),
        ];

        for (input, expected) in cases {
            assert_eq!(super::number(input), expected);
        }
    }
}
