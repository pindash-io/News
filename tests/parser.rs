use std::{clone, fs, io::Write, os::unix::prelude::FileExt};

use anyhow::Result;
use pindash_news::easymark;
use pulldown_cmark::Event;

// https://commonmark.org/help/
// https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax
#[test]
fn parse_entity() -> Result<()> {
    let file = fs::File::open("tests/fixtures/cloudflare.xml")?;
    let feed_rs::model::Feed { entries, .. } = feed_rs::parser::parse(file)?;

    let entity = &entries[0];

    let content = entity
        .content
        .as_ref()
        .and_then(|c| c.body.clone())
        .unwrap_or_default();

    let mut events = Vec::new();
    easymark::parser(content, &mut events);

    dbg!(events);
    Ok(())
}

#[test]
fn parse_simple() -> Result<()> {
    let content = include_str!("fixtures/simple.html");

    let mut events = Vec::new();
    easymark::parser(content, &mut events);

    dbg!(events);
    Ok(())
}

#[test]
fn parse_blockquote() -> Result<()> {
    let content = include_str!("fixtures/blockquote.html");

    let mut events = Vec::new();
    easymark::parser(content, &mut events);

    dbg!(events);
    Ok(())
}

#[test]
fn parse_list() -> Result<()> {
    let content = include_str!("fixtures/list.html");

    let mut events = Vec::new();
    easymark::parser(content, &mut events);

    dbg!(events);
    Ok(())
}

#[test]
fn parse_code() -> Result<()> {
    let content = include_str!("fixtures/code.html");

    let mut events = Vec::new();
    easymark::parser(content, &mut events);

    dbg!(events);
    Ok(())
}

#[test]
fn parse_h_list() -> Result<()> {
    let content = include_str!("fixtures/h_list.html");

    let mut events = Vec::new();
    easymark::parser(content, &mut events);

    dbg!(events);
    Ok(())
}

#[test]
fn parse_escape() -> Result<()> {
    let content = include_str!("fixtures/haskellweekly.atom");

    let mut events = Vec::new();
    easymark::parser(
        dbg!(htmlize::unescape_in(content, htmlize::Context::Attribute)),
        &mut events,
    );

    dbg!(events);
    Ok(())
}

#[test]
fn parse_blog_rust_lang_org() -> Result<()> {
    let content = include_str!("fixtures/blog.rust-lang.org.raw");

    let mut events = Vec::new();
    easymark::parser(
        dbg!(htmlize::unescape_in(content, htmlize::Context::Attribute)),
        &mut events,
    );

    dbg!(events);
    Ok(())
}
