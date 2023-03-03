use anyhow::Result;
use eframe::egui::*;
use pulldown_cmark::{Event, Tag};
use scraper::{Html, Node};

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

    pub indents: usize,
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

pub fn render(ui: &mut Ui, events: Vec<Event<'_>>) -> Result<()> {
    let initial_size = vec2(ui.available_width(), ui.spacing().interact_size.y);
    // ui.visuals_mut().code_bg_color = Color32::from_rgb(255, 0, 0);

    let layout = Layout::left_to_right(Align::BOTTOM).with_main_wrap(true);

    ui.allocate_ui_with_layout(initial_size, layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        let max_height = ui.text_style_height(&TextStyle::Heading);
        let row_height = ui.text_style_height(&TextStyle::Body);
        let one_indent = row_height / 2.0;
        let diff = max_height - row_height;
        let quoted_indent = 2.0 * one_indent + ui.style().spacing.item_spacing.x * 0.5;

        ui.set_row_height(row_height);

        let mut style = Style::default();
        // None: bulleted, Some(n): numbered
        let mut list_type = None;
        let mut is_link = false;
        let mut rich_text = None;

        for event in events {
            item_ui(
                ui,
                &mut style,
                &mut list_type,
                &mut is_link,
                &mut rich_text,
                row_height,
                diff,
                one_indent,
                quoted_indent,
                event,
            );
        }
    });

    Ok(())
}

fn new_line(ui: &mut Ui, row_height: f32, quoted_indent: Option<f32>) {
    ui.allocate_exact_size(
        vec2(quoted_indent.unwrap_or_default(), row_height),
        Sense::hover(),
    ); // make sure we take up some height
    ui.end_row();
    ui.set_row_height(row_height);
}

pub fn item_ui(
    ui: &mut Ui,
    style: &mut Style,
    list_type: &mut Option<u64>,
    is_link: &mut bool,
    rich_text: &mut Option<RichText>,
    row_height: f32,
    diff: f32,
    one_indent: f32,
    quoted_indent: f32,
    event: Event<'_>,
) {
    match event {
        Event::Start(tag) => {
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
                    *is_link = true;
                }

                // block
                Tag::Heading(level, ..) => {
                    new_line(ui, row_height, style.quoted.then_some(quoted_indent));
                    style.heading.replace(level as usize);
                }
                Tag::Paragraph => {
                    if style.quoted {
                        ui.add(Separator::default().vertical());
                    }
                    new_line(ui, row_height, style.quoted.then_some(quoted_indent));
                }
                Tag::BlockQuote => {
                    // new_line(ui, row_height, None);
                    let rect = ui
                        .allocate_exact_size(vec2(2.0 * one_indent, row_height), Sense::hover())
                        .0;
                    let rect = rect.expand2(ui.style().spacing.item_spacing * 0.5);
                    ui.painter().line_segment(
                        [rect.center_top(), rect.center_bottom()],
                        (1.0, ui.visuals().weak_text_color()),
                    );
                    style.quoted = true;
                }
                Tag::List(n) => {
                    // let indents = style.indents as f32 * one_indent;
                    // ui.allocate_exact_size(vec2(indents, row_height), Sense::hover());
                    style.indents += 1;
                    *list_type = n;
                }
                Tag::Item => {
                    new_line(ui, row_height, style.quoted.then_some(quoted_indent));
                    let indents = style.indents;
                    let width = 3.0 * one_indent * indents.saturating_sub(1) as f32;
                    ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
                    let rect = if let Some(number) = list_type.as_mut() {
                        let rect = numbered_point(ui, one_indent, &number.to_string(), row_height);
                        *number += 1;
                        rect
                    } else {
                        bulleted_point(ui, one_indent, row_height)
                    };
                    ui.allocate_exact_size(vec2(rect.width(), row_height), Sense::hover());
                }

                // TODO: download image
                Tag::Image(..) => {
                    new_line(ui, row_height, style.quoted.then_some(quoted_indent));
                }

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
                    *is_link = false;
                    if let Some(rich_text) = rich_text.take() {
                        ui.hyperlink_to(rich_text, href);
                    }
                }

                // block
                Tag::Heading(level, ..) => {
                    style.heading = None;
                }
                Tag::BlockQuote => {
                    style.quoted = false;
                }
                Tag::Paragraph => {
                }
                Tag::List(..) => {
                    // style.indents = style.indents.saturating_sub(1);
                    style.indents -= 1;
                    list_type.take();
                }

                k @ _ => {
                    tracing::trace!("{:?}", k);
                }
            }
        }

        Event::Text(text) => {
            let rt = rich_text_from_style(&text.to_string(), &style, row_height, diff);
            if *is_link {
                rich_text.replace(rt);
            } else {
                ui.label(rt);
            }
        }

        // Inline code
        Event::Code(code) => {
            style.code = true;
            let rt = rich_text_from_style(&code.to_string(), &style, row_height, diff);
            if *is_link {
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
            new_line(ui, row_height, style.quoted.then_some(quoted_indent));
        }
        Event::Rule => {
            ui.add(Separator::default().horizontal());
        }

        k @ _ => {
            tracing::trace!("{:?}", k);
        }
    };
}
