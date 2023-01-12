use std::fs;

use anyhow::Result;
use ego_tree;
use feed_rs;
use pindash_news::easymark;
use pulldown_cmark;
use scraper::{
    self,
    node::{Element, Text},
    Node,
};

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

fn parse_node<'a>(node: ego_tree::NodeRef<'a, Node>) -> Result<()> {
    let value = node.value();
    match value {
        Node::Element(Element {
            name,
            id,
            classes,
            attrs,
        }) => {
            dbg!(name, id);
        }
        Node::Text(Text { text }) => {
            dbg!(text);
        }
        k @ _ => {
            dbg!(k);
        }
    }

    Ok(())
}
