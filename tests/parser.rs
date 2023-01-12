use std::fs;

use anyhow::Result;
use feed_rs;
use pindash_news::easymark;
use scraper;
use ego_tree;
use pulldown_cmark;

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

    let html = easymark::parser(content);

    for node in html.tree.nodes() {
        parse_node(node);
    }

    Ok(())
}

fn parse_node<'a, T: std::fmt::Debug>(node: ego_tree::NodeRef<'a, T>) -> Result<()> {
    dbg!(node.value());
    Ok(())
}
