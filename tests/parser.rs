use std::fs;

use anyhow::Result;
use ego_tree;
use feed_rs;
use pindash_news::easymark;
use pulldown_cmark::{self, Event, HeadingLevel, Tag};
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

    let mut events = Vec::new();
    for node in html.tree.nodes() {
        parse_node(node, &mut events);
    }

    Ok(())
}

fn parse_node<'a>(node: ego_tree::NodeRef<'a, Node>, events: &mut Vec<Event>) -> Result<()> {
    let value = node.value();
    let mut event = None;
    match value {
        Node::Element(element) => {
            match dbg!(element.name()) {
                "h1" => {
                    event.replace(Event::Start(Tag::Heading(HeadingLevel::H1, None, vec![])));
                }
                "h2" => {
                    event.replace(Event::Start(Tag::Heading(HeadingLevel::H2, None, vec![])));
                }
                "h3" => {
                    event.replace(Event::Start(Tag::Heading(HeadingLevel::H3, None, vec![])));
                }
                "h4" => {
                    event.replace(Event::Start(Tag::Heading(HeadingLevel::H4, None, vec![])));
                }
                "h5" => {
                    event.replace(Event::Start(Tag::Heading(HeadingLevel::H5, None, vec![])));
                }
                "h6" => {
                    event.replace(Event::Start(Tag::Heading(HeadingLevel::H6, None, vec![])));
                }
                "p" => {
                    event.replace(Event::Start(Tag::Paragraph));
                }
                "img" => {
                    dbg!(element
                        .attrs()
                        .into_iter()
                        .filter(|a| a.0 == "src" || a.0 == "alt")
                        .map(|a| (a.0.to_string(), a.1.to_string()))
                        .collect::<Vec<_>>());
                    // event.replace(Event::Start(Tag::Image(, , )));
                }
                "blockquote" => {
                    event.replace(Event::Start(Tag::BlockQuote));
                }
                "pre" => {
                }
                "ul" => {
                    event.replace(Event::Start(Tag::List(None)));
                }
                "ol" => {
                    event.replace(Event::Start(Tag::List(Some(0))));
                }
                "li" => {}
                "code" => {}
                "a" => {}
                "strong" => {}
                "em" => {}
                _ => {}
            }
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
