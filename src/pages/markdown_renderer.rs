use std::{cmp, usize};

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Cell, List, ListItem, Paragraph, Row, Table, Widget},
};

use crate::{
    nodes::{RenderComponent, RenderNode, Word, WordType},
    util::CONFIG,
};

fn clips_upper_bound(_area: Rect, component: &RenderComponent) -> bool {
    component.scroll_offset() > component.y_offset()
}

fn clips_lower_bound(area: Rect, component: &RenderComponent) -> bool {
    (component.y_offset() + component.height()).saturating_sub(component.scroll_offset())
        > area.height
}

enum Clipping {
    Both,
    Upper,
    Lower,
    None,
}

impl Widget for RenderComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.y_offset().saturating_sub(self.scroll_offset()) > area.height
            || self.scroll_offset() > self.y_offset() + self.height()
        {
            return;
        }
        let kind = self.kind();

        let y = self.y_offset().saturating_sub(self.scroll_offset());

        let clips = if clips_upper_bound(area, &self) && clips_lower_bound(area, &self) {
            Clipping::Both
        } else if clips_upper_bound(area, &self) {
            Clipping::Upper
        } else if clips_lower_bound(area, &self) {
            Clipping::Lower
        } else {
            Clipping::None
        };

        let height = match clips {
            Clipping::Both => {
                let new_y = self.y_offset().saturating_sub(self.scroll_offset());
                let new_height = new_y;
                cmp::min(self.height(), area.height.saturating_sub(new_height))
            }

            Clipping::Upper => cmp::min(
                self.height(),
                (self.height() + self.y_offset()).saturating_sub(self.scroll_offset()),
            ),
            Clipping::Lower => {
                let new_y = self.y_offset() - self.scroll_offset();
                let new_height = new_y;
                cmp::min(self.height(), area.height.saturating_sub(new_height))
            }
            Clipping::None => self.height(),
        };

        // Just used for task and TODO code block
        let meta_info = self
            .meta_info()
            .to_owned()
            .first()
            .cloned()
            .unwrap_or_else(|| Word::new("".to_string(), WordType::Normal));

        let table_meta = self.meta_info().to_owned();

        let area = Rect { height, y, ..area };

        match kind {
            RenderNode::Paragraph => render_paragraph(area, buf, self, clips),
            RenderNode::Heading => render_heading(area, buf, self),
            RenderNode::Task => render_task(area, buf, self, clips, &meta_info),
            RenderNode::List => render_list(area, buf, self, clips),
            RenderNode::CodeBlock => render_code_block(area, buf, self, clips),
            RenderNode::Table => render_table(area, buf, self, clips, table_meta),
            RenderNode::Quote => render_quote(area, buf, self, clips),
            // RenderNode::Quote => render_quote(area, buf, self.content_owned()),
            RenderNode::LineBreak => (),
            RenderNode::HorizontalSeperator => render_horizontal_seperator(area, buf),
        }
    }
}

fn style_word(word: &Word) -> Span<'_> {
    match word.kind() {
        WordType::MetaInfo(_) | WordType::LinkData => unreachable!(),
        WordType::Selected => Span::styled(
            word.content(),
            Style::default()
                .fg(CONFIG.link_selected_fg_color)
                .bg(CONFIG.link_selected_bg_color),
        ),
        WordType::Normal => Span::raw(word.content()),
        WordType::Code => Span::styled(word.content(), Style::default().fg(CONFIG.code_fg_color))
            .bg(CONFIG.code_bg_color),
        WordType::Link => Span::styled(word.content(), Style::default().fg(CONFIG.link_color)),
        WordType::Italic => Span::styled(
            word.content(),
            Style::default().fg(CONFIG.italic_color).italic(),
        ),
        WordType::Bold => Span::styled(
            word.content(),
            Style::default().fg(CONFIG.bold_color).bold(),
        ),
        WordType::Strikethrough => Span::styled(
            word.content(),
            Style::default()
                .fg(CONFIG.striketrough_color)
                .add_modifier(Modifier::CROSSED_OUT),
        ),
        WordType::White => Span::styled(word.content(), Style::default().fg(Color::White)),
        WordType::ListMarker => Span::styled(word.content(), Style::default().fg(Color::White)),
        WordType::BoldItalic => Span::styled(
            word.content(),
            Style::default()
                .fg(CONFIG.bold_italic_color)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::ITALIC),
        ),
    }
}

fn render_quote(area: Rect, buf: &mut Buffer, component: RenderComponent, clip: Clipping) {
    let top = component
        .scroll_offset()
        .saturating_sub(component.y_offset());
    let mut content = component.content_owned();
    let content = match clip {
        Clipping::Both => {
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
            content
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            let mut content = content;
            content.drain(0..offset);
            content
        }
        Clipping::Lower => {
            let mut content = content;
            content.drain(area.height as usize..);
            content
        }
        Clipping::None => content,
    };

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(|i| style_word(i)).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().style(Style::default().bg(CONFIG.quote_bg_color)));

    let area = Rect {
        width: cmp::min(area.width, CONFIG.width),
        ..area
    };

    paragraph.render(area, buf);
}

fn render_heading(area: Rect, buf: &mut Buffer, component: RenderComponent) {
    let content: Vec<Span<'_>> = component
        .content()
        .iter()
        .flatten()
        .map(|c| Span::styled(c.content(), Style::default().fg(CONFIG.heading_fg_color)))
        .collect();

    let paragraph = Paragraph::new(Line::from(content))
        .block(Block::default().style(Style::default().bg(CONFIG.heading_bg_color)))
        .alignment(Alignment::Center);

    let area = Rect {
        width: cmp::min(area.width, CONFIG.width),
        ..area
    };

    paragraph.render(area, buf);
}

fn render_paragraph(area: Rect, buf: &mut Buffer, component: RenderComponent, clip: Clipping) {
    let top = component
        .scroll_offset()
        .saturating_sub(component.y_offset());
    let mut content = component.content_owned();
    let content = match clip {
        Clipping::Both => {
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
            content
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            let mut content = content;
            content.drain(0..offset);
            content
        }
        Clipping::Lower => {
            let mut content = content;
            content.drain(area.height as usize..);
            content
        }
        Clipping::None => content,
    };

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(|i| style_word(i)).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}

fn render_list(area: Rect, buf: &mut Buffer, component: RenderComponent, clip: Clipping) {
    let top = component
        .scroll_offset()
        .saturating_sub(component.y_offset());
    let mut content = component.content_owned();
    let content = match clip {
        Clipping::Both => {
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
            content
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            let mut content = content;
            content.drain(0..offset);
            content
        }
        Clipping::Lower => {
            let mut content = content;
            content.drain(area.height as usize..);
            content
        }
        Clipping::None => content,
    };
    let content: Vec<ListItem<'_>> = content
        .iter()
        .map(|c| -> ListItem<'_> {
            ListItem::new(Line::from(
                c.iter().map(|i| style_word(i)).collect::<Vec<_>>(),
            ))
        })
        .collect();

    let list = List::new(content);
    list.render(area, buf);
}

fn render_code_block(area: Rect, buf: &mut Buffer, component: RenderComponent, clip: Clipping) {
    let mut content = component
        .content()
        .iter()
        .map(|c| {
            Line::from(
                c.iter()
                    .map(|i| {
                        Span::styled(i.content(), Style::default().fg(CONFIG.code_block_fg_color))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    match clip {
        Clipping::Both => {
            let top = component.scroll_offset() - component.y_offset();
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            // panic!("offset: {}, height: {}, len: {}", offset, height, len);
            content.drain(0..offset);
        }
        Clipping::Lower => {
            content.drain(area.height as usize..);
        }
        Clipping::None => (),
    }

    let area = Rect {
        width: cmp::min(area.width, CONFIG.width),
        ..area
    };

    let block = Block::default().style(Style::default().bg(CONFIG.code_block_bg_color));

    block.render(area, buf);

    let area = Rect {
        x: area.x + 1,
        width: cmp::min(area.width - 2, CONFIG.width),
        ..area
    };

    let paragraph = Paragraph::new(content);

    paragraph.render(area, buf);
}

fn render_table(
    area: Rect,
    buf: &mut Buffer,
    component: RenderComponent,
    clip: Clipping,
    meta_info: Vec<Word>,
) {
    let column_count = meta_info.len();

    let content = component.content();

    let titles = content.chunks(column_count).next().unwrap().to_vec();

    let widths = meta_info
        .iter()
        .map(|c| Constraint::Length(c.content().len() as u16))
        .collect::<Vec<_>>();

    let moved_content = content.chunks(column_count).skip(1).collect::<Vec<_>>();

    let header = Row::new(
        titles
            .iter()
            .map(|c| {
                Cell::from(Line::from(
                    c.iter().map(|i| style_word(i)).collect::<Vec<_>>(),
                ))
            })
            .collect::<Vec<_>>(),
    );

    let mut rows = moved_content
        .iter()
        .map(|c| {
            Row::new(
                c.iter()
                    .map(|i| {
                        Cell::from(Line::from(
                            i.iter().map(|i| style_word(i)).collect::<Vec<_>>(),
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    match clip {
        Clipping::Both => {
            let top = component.scroll_offset() - component.y_offset();
            rows.drain(0..top as usize);
            rows.drain(area.height as usize..);
        }
        Clipping::Upper => {
            let len = rows.len();
            let height = area.height as usize;
            let offset = len.saturating_sub(height) + 1;
            // panic!("offset: {}, height: {}, len: {}", offset, height, len);
            if offset < len {
                rows.drain(0..offset);
            }
        }
        Clipping::Lower => {
            let drain_area = cmp::min(area.height, rows.len() as u16);
            rows.drain(drain_area as usize..);
        }
        Clipping::None => (),
    }

    let table = Table::new(rows, &widths)
        .header(
            header.style(
                Style::default()
                    .fg(CONFIG.table_header_fg_color)
                    .bg(CONFIG.table_header_bg_color),
            ),
        )
        .block(Block::default())
        .column_spacing(1);

    table.render(area, buf);
}

fn render_task(
    area: Rect,
    buf: &mut Buffer,
    component: RenderComponent,
    clip: Clipping,
    meta_info: &Word,
) {
    const CHECKBOX: &str = "✅ ";
    const UNCHECKED: &str = "❌ ";

    let checkbox = if meta_info.content() == "- [ ]" {
        UNCHECKED
    } else {
        CHECKBOX
    };

    let paragraph = Paragraph::new(checkbox);

    paragraph.render(area, buf);

    let area = Rect {
        x: area.x + 4,
        width: area.width - 4,
        ..area
    };

    let top = component
        .scroll_offset()
        .saturating_sub(component.y_offset());

    let mut content = component.content_owned();

    let content = match clip {
        Clipping::Both => {
            content.drain(0..top as usize);
            content.drain(area.height as usize..);
            content
        }
        Clipping::Upper => {
            let len = content.len();
            let height = area.height;
            let offset = len - height as usize;
            let mut content = content;
            content.drain(0..offset);
            content
        }
        Clipping::Lower => {
            let mut content = content;
            content.drain(area.height as usize..);
            content
        }
        Clipping::None => content,
    };

    let lines = content
        .iter()
        .map(|c| Line::from(c.iter().map(|i| style_word(i)).collect::<Vec<_>>()))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines);

    paragraph.render(area, buf);
}

fn render_horizontal_seperator(area: Rect, buf: &mut Buffer) {
    let paragraph = Paragraph::new(Line::from(vec![Span::raw(
        "\u{2014}".repeat(CONFIG.width.into()),
    )]));

    paragraph.render(area, buf);
}
