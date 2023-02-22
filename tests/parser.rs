use std::fs;

use anyhow::Result;
use atoi::ascii_to_digit;
use ego_tree;
use feed_rs;
use pindash_news::easymark;
use pulldown_cmark::{self, CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, Tag};
use scraper::{
    self,
    node::{Element, Text},
    Node,
};

// use html5ever::tendril::StrTendril;

const CRTL: &str = "\n";

// https://commonmark.org/help/
// https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax
#[test]
fn parse_entity() -> Result<()> {
    // let file = fs::File::open("tests/fixtures/cloudflare.xml")?;
    // let feed_rs::model::Feed { entries, .. } = feed_rs::parser::parse(file)?;

    // let entity = &entries[0];

    // let content = entity
    //     .content
    //     .as_ref()
    //     .and_then(|c| c.body.clone())
    //     .unwrap_or_default();
    //

    let content = include_str!("fixtures/simple.html");

    let html = easymark::parser(content);

    let mut events = Vec::<Event<'_>>::new();
    for node in html.root_element().children() {
        // blocks
        match node.value() {
            Node::Element(elem) => {
                let name = elem.name();
                match name {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        let n = ascii_to_digit::<usize>(name.bytes().nth(1).unwrap()).unwrap();
                        let tag =
                            Tag::Heading(HeadingLevel::try_from(n).unwrap(), None, Vec::new());
                        events.push(Event::Start(tag.clone()));

                        parse_inline(&mut events, node);

                        events.push(Event::End(tag));
                    }
                    "p" => {
                        let tag = Tag::Paragraph;
                        events.push(Event::Start(tag.clone()));

                        parse_inline(&mut events, node);

                        events.push(Event::End(tag));
                    }
                    "br" => {
                        events.push(Event::HardBreak);
                    }
                    "img" => {
                        let mut attrs = elem
                            .attrs()
                            .into_iter()
                            .filter(|a| a.0 == "src" || a.0 == "alt")
                            .map(|a| (a.0.to_string(), a.1.to_string()))
                            .collect::<Vec<_>>();

                        attrs.sort_by_key(|attr| attr.0.clone());

                        if attrs.is_empty() {
                            continue;
                        }

                        let (src, alt) = (
                            attrs[0].1.clone(),
                            if attrs.len() == 1 {
                                String::new()
                            } else {
                                attrs[1].1.clone()
                            },
                        );

                        let tag = Tag::Image(LinkType::Inline, src.into(), alt.into());
                        events.push(Event::Start(tag.clone()));
                        events.push(Event::End(tag));
                    }
                    "blockquote" => {
                        let tag = Tag::BlockQuote;
                        events.push(Event::Start(tag.clone()));

                        parse_inline(&mut events, node);

                        events.push(Event::End(tag));
                    }
                    "ol" | "ul" => {
                        parse_list(
                            &mut events,
                            node,
                            (name.chars().next() == Some('o')).then_some(0),
                        );
                    }
                    "hr" => {
                        events.push(Event::Rule);
                    }
                    // "pre" => {}
                    // "code" => {
                    //     event.replace(Event::Code(CowStr::Borrowed("")));
                    // }
                    _ => {}
                }
            }
            Node::Text(Text { text }) if text.trim_end_matches(' ') == CRTL => {
                // events.push(Event::SoftBreak)
            }
            _ => {}
        }
    }

    dbg!(events);
    Ok(())
}

fn parse_list<'a>(
    events: &mut Vec<Event<'a>>,
    parent: ego_tree::NodeRef<'a, Node>,
    kind: Option<u64>,
) {
    let tag = Tag::List(kind);
    events.push(Event::Start(tag.clone()));

    for node in parent.children() {
        match node.value() {
            Node::Element(elem) if elem.name() == "li" => {
                let tag = Tag::Item;
                events.push(Event::Start(tag.clone()));

                for sub_node in node.children() {
                    match sub_node.value() {
                        // nested list
                        Node::Element(sub_elem) => {
                            if matches!(sub_elem.name(), "ol" | "ul") {
                                parse_list(
                                    events,
                                    sub_node,
                                    (sub_elem.name().chars().next() == Some('o')).then_some(0),
                                );
                            }
                        }
                        _ => {
                            parse_inline(events, node);
                        }
                    }
                }

                events.push(Event::End(tag));
            }
            _ => {}
        }
    }

    events.push(Event::End(tag));
}

fn parse_inline<'a>(events: &mut Vec<Event<'a>>, parent: ego_tree::NodeRef<'a, Node>) {
    for node in parent.children() {
        match node.value() {
            Node::Element(elem) => {
                let name = elem.name();
                match name {
                    // Link
                    "a" => {
                        let mut attrs = elem
                            .attrs()
                            .into_iter()
                            .filter(|a| a.0 == "href" || a.0 == "title")
                            .map(|a| (a.0.to_string(), a.1.to_string()))
                            .collect::<Vec<_>>();

                        attrs.sort_by_key(|attr| attr.0.clone());

                        if attrs.is_empty() {
                            continue;
                        }

                        let (href, title) = (
                            attrs[0].1.clone(),
                            if attrs.len() == 1 {
                                String::new()
                            } else {
                                attrs[1].1.clone()
                            },
                        );

                        let tag = Tag::Link(LinkType::Inline, href.into(), title.into());
                        events.push(Event::Start(tag.clone()));

                        parse_inline(events, node);

                        events.push(Event::End(tag));
                    }
                    // Blod
                    "strong" => {
                        let tag = Tag::Strong;
                        events.push(Event::Start(tag.clone()));

                        parse_inline(events, node);

                        events.push(Event::End(tag));
                    }
                    // Italic
                    "em" => {
                        let tag = Tag::Emphasis;
                        events.push(Event::Start(tag.clone()));

                        parse_inline(events, node);

                        events.push(Event::End(tag));
                    }
                    // Strikethrough
                    "del" => {
                        let tag = Tag::Strikethrough;
                        events.push(Event::Start(tag.clone()));

                        parse_inline(events, node);

                        events.push(Event::End(tag));
                    }
                    // Inline Code
                    "code" => {
                        if let Some(Text { text }) =
                            node.first_child().and_then(|node| node.value().as_text())
                        {
                            let event = Event::Code(CowStr::Borrowed(text));
                            events.push(event);
                        }
                    }
                    // Subscript
                    // "sub" => {},
                    // Superscript
                    // "sup" => {},
                    _ => {}
                }
            }
            Node::Text(Text { text }) if text.trim_end_matches(' ') != CRTL => {
                events.push(Event::Text(CowStr::Borrowed(text)));
            }
            _ => {}
        }
    }
}
