use std::ops::Deref;

use eframe::egui::*;
use scraper::{Html, Node};

pub fn render(ui: &mut Ui, html: Html) {
    let initial_size = vec2(
        ui.available_width(),
        ui.spacing().interact_size.y, // Assume there will be
    );

    let layout = Layout::left_to_right(Align::BOTTOM).with_main_wrap(true);

    ui.allocate_ui_with_layout(initial_size, layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let row_height = ui.text_style_height(&TextStyle::Body);
        ui.set_row_height(row_height);

        html.tree
            .nodes()
            .into_iter()
            .map(|item| {
                let value = item.value();
                match value {
                    Node::Element(elem) => {
                        elem.name();
                    }
                    Node::Text(text) => {
                        let t = text.deref();
                        // dbg!(t);
                        item_ui(ui, t);
                    }
                    _ => {}
                }
                if item.has_children() {
                    item.children()
                        .into_iter()
                        .map(|item| {
                            let value = item.value();
                            match value {
                                Node::Element(elem) => {
                                    elem.name();
                                }
                                Node::Text(text) => {
                                    let t = text.deref();
                                    // dbg!(t);
                                    item_ui(ui, t);
                                }
                                _ => {}
                            }
                        })
                        .collect::<Vec<_>>();
                }
            })
            .collect::<Vec<_>>();
    });
}

pub fn item_ui(ui: &mut Ui, text: &str) {
    // ui.label(rich_text_from_style(text, &style));
    ui.label(rich_text_from_style(text, &Style::default()));
}

// pub fn item_ui(ui: &mut Ui, item: easy_mark::Item<'_>) {
//     let row_height = ui.text_style_height(&TextStyle::Body);
//     let one_indent = row_height / 2.0;

//     match item {
//         easy_mark::Item::Newline => {
//             // ui.label("\n"); // too much spacing (paragraph spacing)
//             ui.allocate_exact_size(vec2(0.0, row_height), Sense::hover()); // make sure we take up some height
//             ui.end_row();
//             ui.set_row_height(row_height);
//         }

//         easy_mark::Item::Text(style, text) => {
//             ui.label(rich_text_from_style(text, &style));
//         }
//         easy_mark::Item::Hyperlink(style, text, url) => {
//             let label = rich_text_from_style(text, &style);
//             ui.add(Hyperlink::from_label_and_url(label, url));
//         }

//         easy_mark::Item::Separator => {
//             ui.add(Separator::default().horizontal());
//         }
//         easy_mark::Item::Indentation(indent) => {
//             let indent = indent as f32 * one_indent;
//             ui.allocate_exact_size(vec2(indent, row_height), Sense::hover());
//         }
//         easy_mark::Item::QuoteIndent => {
//             let rect = ui
//                 .allocate_exact_size(vec2(2.0 * one_indent, row_height), Sense::hover())
//                 .0;
//             let rect = rect.expand2(ui.style().spacing.item_spacing * 0.5);
//             ui.painter().line_segment(
//                 [rect.center_top(), rect.center_bottom()],
//                 (1.0, ui.visuals().weak_text_color()),
//             );
//         }
//         easy_mark::Item::BulletPoint => {
//             ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
//             bullet_point(ui, one_indent);
//             ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
//         }
//         easy_mark::Item::NumberedPoint(number) => {
//             let width = 3.0 * one_indent;
//             numbered_point(ui, width, number);
//             ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
//         }
//         easy_mark::Item::CodeBlock(_language, code) => {
//             let where_to_put_background = ui.painter().add(Shape::Noop);
//             let mut rect = ui.monospace(code).rect;
//             rect = rect.expand(1.0); // looks better
//             rect.max.x = ui.max_rect().max.x;
//             let code_bg_color = ui.visuals().code_bg_color;
//             ui.painter().set(
//                 where_to_put_background,
//                 Shape::rect_filled(rect, 1.0, code_bg_color),
//             );
//         }
//     };
// }

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
    /// # heading (large text)
    pub heading: bool,

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
}

fn rich_text_from_style(text: &str, style: &Style) -> RichText {
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
    } = *style;

    let small = small || raised; // Raised text is also smaller

    let mut rich_text = RichText::new(text);
    if heading && !small {
        rich_text = rich_text.heading().strong();
    }
    if small && !heading {
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

fn bullet_point(ui: &mut Ui, width: f32) -> Response {
    let row_height = ui.text_style_height(&TextStyle::Body);
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    ui.painter().circle_filled(
        rect.center(),
        rect.height() / 8.0,
        ui.visuals().strong_text_color(),
    );
    response
}

fn numbered_point(ui: &mut Ui, width: f32, number: &str) -> Response {
    let font_id = TextStyle::Body.resolve(ui.style());
    let row_height = ui.fonts().row_height(&font_id);
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    let text = format!("{}.", number);
    let text_color = ui.visuals().strong_text_color();
    ui.painter().text(
        rect.right_center(),
        Align2::RIGHT_CENTER,
        text,
        font_id,
        text_color,
    );
    response
}
