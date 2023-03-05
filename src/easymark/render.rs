use anyhow::Result;
use eframe::egui::*;
use pulldown_cmark::{CodeBlockKind, Event, Tag};
use scraper::{Html, Node};

use super::code_view_ui;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
    /// # heading (large text)
    pub heading: Option<usize>,

    /// > quoted (slightly dimmer color or other font style)
    pub quoted: bool,

    /// `code` (monospace, some other color)
    pub code: bool,

    /// self.strong* (emphasized, e.g. bold)
    pub strong: bool,

    /// _underline_
    pub underline: bool,

    /// ~strikethrough~
    pub strikethrough: bool,

    /// /italics/
    pub italics: bool,

    /// $small$
    pub small: bool,

    /// ^raised^
    pub raised: bool,

    pub link: bool,

    pub first_item: bool,

    pub codeblock: bool,
}

impl Style {
    pub fn inline(&self) -> bool {
        self.strong
            || self.underline
            || self.strikethrough
            || self.italics
            || self.small
            || self.raised
            || self.link
    }
}

fn rich_text_from_style(text: &str, style: &Style, row_height: f32, diff: f32) -> RichText {
    let Style {
        heading,
        quoted,
        code,
        strong,
        underline,
        strikethrough,
        italics,
        small,
        raised,
        ..
    } = *style;

    let small = small || raised; // Raised text is also smaller

    let mut rich_text = RichText::new(text);

    if let Some(level) = heading {
        match level {
            1 => {
                rich_text = rich_text.strong().heading();
            }
            k @ 2..=6 => {
                let size = row_height
                    + diff
                        * (match k {
                            2 => 0.835,
                            3 => 0.668,
                            4 => 0.501,
                            5 => 0.334,
                            6 => 0.167,
                            _ => unreachable!(),
                        });
                rich_text = rich_text.strong().size(size);
            }
            _ => {
                unreachable!();
            }
        }
    }

    if small && heading.is_none() {
        rich_text = rich_text.small();
    }
    if code {
        rich_text = rich_text.code();
    }
    if strong {
        rich_text = rich_text.strong();
    } else if quoted {
        rich_text = rich_text.weak();
    }
    if underline {
        rich_text = rich_text.underline();
    }
    if strikethrough {
        rich_text = rich_text.strikethrough();
    }
    if italics {
        rich_text = rich_text.italics();
    }
    if raised {
        rich_text = rich_text.raised();
    }
    rich_text
}

fn bulleted_point(ui: &mut Ui, width: f32, row_height: f32) -> Rect {
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    ui.painter().circle_filled(
        rect.center(),
        rect.height() / 8.0,
        ui.visuals().strong_text_color(),
    );
    rect
}

fn numbered_point(ui: &mut Ui, width: f32, number: &str, row_height: f32) -> Rect {
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    let mut text = String::new();
    text.push_str(&number.to_string());
    text.push('.');
    let text_color = ui.visuals().strong_text_color();
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        text,
        TextStyle::Body.resolve(ui.style()),
        text_color,
    )
}

fn new_line(ui: &mut Ui, row_height: f32) {
    ui.allocate_exact_size(vec2(0.0, row_height), Sense::hover()); // make sure we take up some height
    ui.end_row();
    ui.set_row_height(row_height);
}

pub fn render(ui: &mut Ui, events: Vec<Event<'_>>) -> Result<()> {
    let initial_size = vec2(ui.available_width(), ui.spacing().interact_size.y);
    let layout = Layout::left_to_right(Align::BOTTOM).with_main_wrap(true);

    ui.allocate_ui_with_layout(initial_size, layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let max_height = ui.text_style_height(&TextStyle::Heading);
        let row_height = ui.text_style_height(&TextStyle::Body);
        let one_indent = row_height / 2.0;
        let diff = max_height - row_height;

        ui.set_row_height(row_height);

        let mut style = Style::default();
        // None: bulleted, Some(n): numbered
        let mut list = None;
        let mut rich_text = None;
        let mut lang = None;

        for event in events {
            item_ui(
                ui,
                &mut style,
                &mut list,
                &mut rich_text,
                &mut lang,
                row_height,
                diff,
                one_indent,
                event,
            );
        }
    });

    Ok(())
}

pub fn item_ui(
    ui: &mut Ui,
    style: &mut Style,
    list: &mut Option<Vec<Option<u64>>>,
    rich_text: &mut Option<RichText>,
    lang: &mut Option<String>,
    row_height: f32,
    diff: f32,
    one_indent: f32,
    event: Event<'_>,
) {
    match event {
        Event::Start(tag) => {
            // draw quoted
            if style.quoted && !style.inline() && !matches!(tag, Tag::List { .. }) {
                let rect = ui
                    .allocate_exact_size(vec2(1.5 * one_indent, row_height), Sense::hover())
                    .0;
                let rect = rect.expand2(ui.style().spacing.item_spacing * 0.5);
                ui.painter().line_segment(
                    [rect.center_top(), rect.center_bottom()],
                    (1.0, ui.visuals().weak_text_color()),
                );
            }

            match tag {
                // inline
                Tag::Strong => {
                    style.strong = true;
                }
                Tag::Emphasis => {
                    style.italics = true;
                }
                Tag::Strikethrough => {
                    style.strikethrough = true;
                }
                Tag::Link(..) => {
                    style.link = true;
                }

                // block
                Tag::Heading(level, ..) => {
                    style.heading.replace(level as usize);
                }
                Tag::BlockQuote => {
                    style.quoted = true;
                }
                Tag::Paragraph => {}
                Tag::List(n) => {
                    let list = list.get_or_insert_with(Vec::new);
                    if list.is_empty() {
                        style.first_item = true;
                    }
                    list.push(n);
                }
                Tag::Item => {
                    let list = list.get_or_insert_with(Vec::new);
                    let indents = list.len();
                    let kind = list.last_mut();

                    if !style.quoted && !style.first_item {
                        new_line(ui, row_height);
                    }

                    let width = 3.0 * one_indent * indents.saturating_sub(1) as f32;

                    ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
                    let rect = if let Some(Some(number)) = kind {
                        let rect = numbered_point(ui, one_indent, &number.to_string(), row_height);
                        *number += 1;
                        rect
                    } else {
                        bulleted_point(ui, one_indent, row_height)
                    };
                    ui.allocate_exact_size(vec2(rect.width(), row_height), Sense::hover());
                }
                Tag::CodeBlock(kind) => {
                    style.codeblock = true;
                    match kind {
                        CodeBlockKind::Indented => *lang = None,
                        CodeBlockKind::Fenced(text) => {
                            lang.replace(text.to_string());
                        }
                    }
                }

                // TODO: download image
                Tag::Image(..) => {}

                k @ _ => {
                    tracing::trace!("{:?}", k);
                }
            }
        }
        Event::End(tag) => {
            // inline
            match tag {
                Tag::Strong => {
                    style.strong = false;
                }
                Tag::Emphasis => {
                    style.italics = false;
                }
                Tag::Strikethrough => {
                    style.strikethrough = false;
                }
                Tag::Link(_, href, _) => {
                    style.link = false;
                    if let Some(rich_text) = rich_text.take() {
                        ui.hyperlink_to(rich_text, href);
                    }
                }
                // block
                Tag::Heading(..) => {
                    style.heading = None;
                    new_line(ui, row_height);
                }
                Tag::List(..) => {
                    let list = list.get_or_insert_with(Vec::new);
                    list.pop();
                    if !style.quoted && list.is_empty() {
                        new_line(ui, row_height);
                    }
                }
                Tag::BlockQuote => {
                    style.quoted = false;
                    // new_line(ui, row_height);
                }
                Tag::Paragraph => {
                    new_line(ui, row_height);
                }
                Tag::Image(..) => {
                    new_line(ui, row_height);
                }
                Tag::Item => {
                    if style.first_item {
                        style.first_item = false;
                    }
                    if style.quoted {
                        new_line(ui, row_height);
                    }
                }
                Tag::CodeBlock(..) => {
                    style.codeblock = false;
                    new_line(ui, row_height);
                }

                k @ _ => {
                    tracing::trace!("{:?}", k);
                }
            }
        }

        Event::Text(text) => {
            let mut text = text.to_string();
            if style.codeblock {
                code_view_ui(ui, &text, &lang.take().unwrap_or_default());
                return;
            }
            if style.quoted {
                text = text.trim_matches('\n').to_string();
            }
            let rt = rich_text_from_style(&text, &style, row_height, diff);
            if style.link {
                rich_text.replace(rt);
            } else {
                ui.label(rt);
            }
        }

        // Inline code
        Event::Code(code) => {
            style.code = true;
            let mut text = code.to_string();
            if style.quoted {
                text = text.trim_matches('\n').to_string();
            }
            let rt = rich_text_from_style(&text, &style, row_height, diff);
            if style.link {
                rich_text.replace(rt);
            } else {
                ui.label(rt);
            }
            style.code = false;
        }

        Event::SoftBreak => {
            ui.label(" ");
        }
        Event::HardBreak => {
            new_line(ui, row_height);
        }
        Event::Rule => {
            ui.add(Separator::default().horizontal());
        }

        k @ _ => {
            tracing::trace!("{:?}", k);
        }
    };
}
