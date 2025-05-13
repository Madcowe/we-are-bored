use bored::notice::{Display, get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, Coordinate};
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::style::{Styled, Stylize};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget, Wrap},
};
use std::cmp::{max, min};

use crate::app::{App, CreateMode, DraftMode, GoToMode, HyperlinkMode, View};

/// Represent the layout of the bored an it's notices in rects
struct BoredOfRects {
    // bored: Rect,
    notice_rects: Vec<Rect>,
}

impl BoredOfRects {
    fn create(bored: &Bored, y_offset: u16) -> BoredOfRects {
        let mut notice_rects = vec![];
        for notice in bored.get_notices() {
            let notice_rect = Rect::new(
                notice.get_top_left().x,
                notice.get_top_left().y + y_offset,
                notice.get_dimensions().x,
                notice.get_dimensions().y + y_offset,
            );
            notice_rects.push(notice_rect);
        }
        // let bored_rect = Rect::new(
        //     0,
        //     y_offset,
        //     bored.get_dimensions().x,
        //     bored.get_dimensions().y + y_offset,
        // );
        BoredOfRects {
            // bored: bored_rect,
            notice_rects,
        }
    }

    /// returns a vector of blocks with the notice text attached to the rects
    /// inluding underline for hyperlinks, hower new line sin the text will be lost
    fn get_display_notices(&self, bored: &Bored) -> Result<Vec<(Paragraph, Rect)>, BoredError> {
        let mut display_notices = vec![];
        let hyperlink_style = Style::new().underlined();
        let notices = bored
            .get_notices()
            .into_iter()
            .zip(self.notice_rects.clone());
        for (notice, notice_rect) in notices {
            let display = get_display(notice.get_content(), get_hyperlinks(notice.get_content())?);
            let block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default());
            let display_text = display.get_display_text();
            let mut end_of_previous_span = 0;
            let mut chars_in_previous_lines = 0;
            let mut lines = vec![];
            for line in display_text.lines() {
                let mut spans = vec![];
                for hyperlink_location in display.get_hyperlink_locations().into_iter() {
                    // if hyperlinks is on that line...it may span several
                    let line_end = chars_in_previous_lines + line.len();
                    if hyperlink_location.1 > chars_in_previous_lines
                        && hyperlink_location.0 < line_end
                    {
                        // set preceding non hyperlinked bit
                        if hyperlink_location.0 > end_of_previous_span {
                            let start = max(end_of_previous_span, chars_in_previous_lines);
                            let end = min(hyperlink_location.0, line_end);
                            let span_text = display_text[start..end].to_owned();
                            let span = Span::styled(span_text, Style::default());
                            spans.push(span);
                        }
                        // set hyperlinked bit
                        let start = max(hyperlink_location.0, chars_in_previous_lines);
                        let end = min(hyperlink_location.1, line_end);
                        let span_text = display_text[start..end].to_owned();
                        let span = Span::styled(span_text, hyperlink_style);
                        spans.push(span);
                        end_of_previous_span = end;
                        // set bit after final hyperlink if there is one
                        if end_of_previous_span < line_end {
                            let end = min(line_end, display_text.len());
                            let span_text = display_text[end_of_previous_span..end].to_owned();
                            let span = Span::styled(span_text, Style::default());
                            spans.push(span);
                        }
                    }
                }
                chars_in_previous_lines += line.len() + 1;
                let line = Line::from_iter(spans.to_owned());
                lines.push(spans);
            }
            let mut text = Text::from(display_text);
            if !display.get_hyperlink_locations().is_empty() {
                text = Text::from_iter(lines);
            }
            let paragraph = Paragraph::new(text).block(block.clone()).white();
            display_notices.push((paragraph, notice_rect));
        }
        Ok(display_notices)
    }
}

/// widget that can render the entirety of a bored
pub struct DisplayBored {
    bored: Bored,
    // bored_rect: Rect,
}
impl Widget for DisplayBored {
    fn render(self, _: Rect, buffer: &mut Buffer) {
        let bored_of_rects = BoredOfRects::create(&self.bored, 0);
        if let Ok(display_notices) = bored_of_rects.get_display_notices(&self.bored) {
            for (display_notice, notice_rect) in display_notices {
                Clear.render(notice_rect, buffer);
                display_notice.render(notice_rect, buffer);
            }
        }
    }
}

impl DisplayBored {
    pub fn create(bored: &Bored) -> DisplayBored {
        // let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
        DisplayBored {
            bored: bored.clone(),
            // bored_rect,
        }
    }
}

/// Widget to display a part of the bored that can fit in the ui depedning on the terminal size
/// with methods to move the view about the bored if it can't all be seen at once
pub struct BoredViewPort {
    bored: Bored,
    bored_rect: Rect,
    bored_dimensions: Coordinate,
    view_top_left: Coordinate,
    view_dimensions: Coordinate,
    buffer: Buffer,
}
impl Widget for BoredViewPort {
    fn render(mut self, view_rect: Rect, buffer: &mut Buffer) {
        let bored_rect = Rect::new(
            0,
            0,
            self.bored.get_dimensions().x,
            self.bored.get_dimensions().y,
        );
        let display_bored = DisplayBored::create(&self.bored);
        display_bored.render(bored_rect, &mut self.buffer);
        let visible_content = self.buffer.content;
        for x in view_rect.x..view_rect.width {
            for y in view_rect.y..view_rect.height {
                let bored_x = x + view_rect.x;
                let bored_y = y + view_rect.y;
                let bored_pos = bored_y * self.bored_rect.width + bored_x;
                buffer[(x, y)] = visible_content[bored_pos as usize].clone();
            }
        }
    }
}

impl BoredViewPort {
    pub fn create(bored: &Bored, view_dimensions: Coordinate) -> BoredViewPort {
        let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
        // if view dimension is lerge then bored dimension user bored dimension
        let view_dimensions = if view_dimensions.within(&bored.get_dimensions()) {
            view_dimensions
        } else {
            bored.get_dimensions()
        };
        BoredViewPort {
            bored: bored.clone(),
            bored_rect,
            bored_dimensions: bored.get_dimensions(),
            view_top_left: Coordinate { x: 0, y: 0 },
            view_dimensions,
            buffer: Buffer::empty(bored_rect),
        }
    }

    /// Moves the view, if view would place any part if the view outside the bored nothing happens
    pub fn move_view(&mut self, view_top_left: Coordinate) {
        if view_top_left
            .add(&self.view_dimensions)
            .within(&self.bored_dimensions)
        {
            self.view_top_left = view_top_left;
        }
    }

    /// Get rect that is the size of the view
    pub fn get_view_rect(&self) -> Rect {
        Rect::new(0, 0, self.view_dimensions.x, self.view_dimensions.y)
    }
}

#[cfg(test)]

mod tests {

    use bored::notice::Notice;

    use super::*;

    #[test]
    fn test_bored_of_rects() -> Result<(), BoredError> {
        let mut bored = Bored::create("Hello", Coordinate { x: 120, y: 40 });
        let bored_of_rects = BoredOfRects::create(&bored, 0);
        // assert_eq!(bored_of_rects.bored, Rect::new(0, 0, 120, 40));
        assert!(bored_of_rects.notice_rects.is_empty());
        let notice = Notice::create(Coordinate { x: 60, y: 18 });
        bored.add(notice, Coordinate { x: 10, y: 5 })?;
        let bored_of_rects = BoredOfRects::create(&bored, 0);
        assert_eq!(bored_of_rects.notice_rects[0].x, 10);
        assert_eq!(bored_of_rects.notice_rects[0].y, 5);
        assert_eq!(bored_of_rects.notice_rects[0].width, 60);
        assert_eq!(bored_of_rects.notice_rects[0].height, 18);
        Ok(())
    }

    #[test]
    fn test_get_display_notices() -> Result<(), BoredError> {
        let mut bored = Bored::create("Hello", Coordinate { x: 120, y: 40 });
        let bored_of_rects = BoredOfRects::create(&bored, 0);
        let display_notices = bored_of_rects.get_display_notices(&bored)?;
        assert!(display_notices.is_empty());
        let notice = Notice::create(Coordinate { x: 60, y: 18 });
        bored.add(notice, Coordinate { x: 10, y: 5 })?;
        let bored_of_rects = BoredOfRects::create(&bored, 0);
        let display_notices = bored_of_rects.get_display_notices(&bored)?;
        assert_eq!(display_notices.len(), 1);
        Ok(())
    }

    #[test]
    fn test_display_bored_render() -> Result<(), BoredError> {
        let mut bored = Bored::create("Hello", Coordinate { x: 60, y: 20 });
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write(
            "We are [link](url) bored.\nYou are [link](url) bored.\nI am [boooo\nooored](url)",
        )?;
        bored.add(notice, Coordinate { x: 5, y: 3 })?;
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write("world")?;
        bored.add(notice, Coordinate { x: 30, y: 10 })?;
        let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
        let mut buffer = Buffer::empty(bored_rect);
        let display_bored = DisplayBored::create(&bored);
        display_bored.render(bored_rect, &mut buffer);
        eprintln!("{:?}", buffer);
        let expected_output = r#"Buffer {
    area: Rect { x: 0, y: 0, width: 60, height: 20 },
    content: [
        "                                                            ",
        "                                                            ",
        "                                                            ",
        "     ┌────────────────────────────┐                         ",
        "     │We are link bored.          │                         ",
        "     │You are link bored.         │                         ",
        "     │I am boooo                  │                         ",
        "     │ooored                      │                         ",
        "     │                            │                         ",
        "     │                            │                         ",
        "     │                        ┌────────────────────────────┐",
        "     └────────────────────────│world                       │",
        "                              │                            │",
        "                              │                            │",
        "                              │                            │",
        "                              │                            │",
        "                              │                            │",
        "                              │                            │",
        "                              └────────────────────────────┘",
        "                                                            ",
    ],
    styles: [
        x: 0, y: 0, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 3, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 35, y: 3, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 4, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 13, y: 4, fg: White, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 17, y: 4, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 35, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 5, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 14, y: 5, fg: White, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 18, y: 5, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 35, y: 5, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 6, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 11, y: 6, fg: White, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 16, y: 6, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 35, y: 6, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 7, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 6, y: 7, fg: White, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 12, y: 7, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 35, y: 7, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 8, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 35, y: 8, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 9, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 35, y: 9, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 10, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 11, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 5, y: 11, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 12, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 30, y: 12, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 13, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 30, y: 13, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 14, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 30, y: 14, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 15, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 30, y: 15, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 16, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 30, y: 16, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 17, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 30, y: 17, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 18, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 30, y: 18, fg: White, bg: Reset, underline: Reset, modifier: NONE,
        x: 0, y: 19, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
    ]
}"#;
        assert_eq!(expected_output, format!("{:?}", buffer));
        // just test view port with 100% view so should be the same as above
        let mut bored_view_port = BoredViewPort::create(&bored, Coordinate { x: 40, y: 15 });
        // bored_view_port.move_view(Coordinate { x: 5, y: 12 });
        let mut buffer = Buffer::empty(bored_rect);
        bored_view_port.render(bored_rect, &mut buffer);
        eprintln!("{:?}", buffer);
        Ok(())
    }
}
