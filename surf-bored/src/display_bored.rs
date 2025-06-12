use bored::notice::{Display, Notice, NoticeHyperlinkMap, get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, BoredHyperlinkMap, Coordinate, bored_client};
use rand::seq::IndexedRandom;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Position;
use ratatui::style::{Styled, Stylize};
use ratatui::text::ToSpan;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Widget, Wrap},
};
use std::cmp::{max, min};

use crate::app::{App, CreateMode, DraftMode, GoToMode, HyperlinkMode, Theme, View};

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
        BoredOfRects { notice_rects }
    }

    /// returns a vector of blocks with the notice text attached to the rects
    /// inluding styling for hyperlinks, however new lines in the text will be lost
    fn get_display_notices(
        &self,
        bored: &Bored,
        // hyperlink_style: Style,
    ) -> Result<Vec<(Paragraph, Rect)>, BoredError> {
        let mut display_notices = vec![];
        let notices = bored
            .get_notices()
            .into_iter()
            .zip(self.notice_rects.clone());
        for (notice, notice_rect) in notices {
            let display = get_display(notice.get_content(), get_hyperlinks(notice.get_content())?);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick);
            let text = character_wrap(display.get_display_text(), notice.get_text_width());
            let paragraph = Paragraph::new(text).block(block.clone()).white();
            display_notices.push((paragraph, notice_rect));
        }
        Ok(display_notices)
    }
}

/// widget that can render the entirety of a bored
pub struct DisplayBored {
    bored: Bored,
    theme: Theme,
}
impl Widget for DisplayBored {
    fn render(self, _: Rect, buffer: &mut Buffer) {
        // Render background of bored
        let bored_block = Block::default()
            .borders(Borders::ALL)
            .style(self.theme.text_style())
            .border_type(BorderType::Rounded);
        bored_block.render(buffer.area, buffer);
        let bored_of_rects = BoredOfRects::create(&self.bored, 0);
        if let Ok(display_notices) = bored_of_rects.get_display_notices(&self.bored) {
            for (display_notice, notice_rect) in display_notices {
                let display_notice = display_notice.clone().set_style(self.theme.text_style());
                Clear.render(notice_rect, buffer);
                display_notice.render(notice_rect, buffer);
            }
            // style hyperlinks
            style_bored_hyperlinks(&self.bored, buffer, self.theme.hyperlink_style());
        }
    }
}

impl DisplayBored {
    pub fn create(bored: &Bored, theme: Theme) -> DisplayBored {
        DisplayBored {
            bored: bored.clone(),
            theme,
        }
    }
}

/// Widget to display a part of the bored that can fit in the ui depending on the terminal size
/// with methods to move the view about the bored if it can't all be seen at once
#[derive(Debug)]
pub struct BoredViewPort {
    bored: Bored,
    bored_rect: Rect,
    bored_dimensions: Coordinate,
    view_top_left: Coordinate,
    view_dimensions: Coordinate,
    buffer: Buffer,
}

impl BoredViewPort {
    pub fn create(bored: &Bored, view_dimensions: Coordinate) -> BoredViewPort {
        let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
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
        self.view_top_left = view_top_left;
    }

    /// checks if bottom righ is within view, so can test wether the view needs to scroll
    pub fn in_view(&self, bottom_right: Coordinate) -> bool {
        if bottom_right.within(&self.view_dimensions) {
            true
        } else {
            false
        }
    }

    pub fn get_view_top_left(&self) -> Coordinate {
        self.view_top_left
    }

    /// Get rect that is position and size of view
    pub fn get_view(&self) -> Rect {
        Rect::new(
            self.view_top_left.x,
            self.view_top_left.y,
            self.view_dimensions.x,
            self.view_dimensions.y,
        )
    }

    /// Change size of view port can be larger than bored
    pub fn set_view_dimensions(&mut self, view_dimensions: Coordinate) {
        self.view_dimensions = view_dimensions;
    }

    /// render just what is in the view port
    pub fn render_view(&mut self, buffer: &mut Buffer, theme: Theme) {
        let view_rect = self.get_view();
        let buffer_rect = buffer.area().clone();
        let x_limit = view_rect.x
            + min(
                view_rect.width,
                min(buffer_rect.width, self.bored_rect.width),
            );
        let y_limit = view_rect.y
            + min(
                view_rect.height,
                min(buffer_rect.height, self.bored_rect.height),
            );
        let display_bored = DisplayBored::create(&self.bored, theme.clone());
        display_bored.render(self.bored_rect, &mut self.buffer);
        let bored_content = self.buffer.content.clone();
        for x in view_rect.x..x_limit {
            for y in view_rect.y..y_limit {
                let bored_pos = y * self.bored_rect.width + x;
                if bored_pos < bored_content.len() as u16 {
                    buffer[(x + buffer_rect.x, y + buffer_rect.y)] =
                        bored_content[bored_pos as usize].clone();
                }
            }
        }
    }
}

/// Wrap text on a character basis so word can be on mutiple lines using ratatui text hierachy
pub fn character_wrap(display_text: String, line_width: u16) -> Text<'static> {
    let mut lines = vec![];
    let mut line = Line::raw("");
    let mut line_char_index = 0;
    for char in display_text.chars() {
        // if line_char % line_width as usize == 0 && char_index > 0 {
        if char == '\n' {
            lines.push(line);
            line = Line::raw("");
            line_char_index = 0;
        } else if line_char_index < line_width {
            line.push_span(Span::raw(char.to_string()));
            line_char_index += 1;
        } else {
            lines.push(line);
            line = Line::raw("");
            line_char_index = 0;
            line.push_span(Span::raw(char.to_string()));
            line_char_index += 1;
        }
    }
    lines.push(line);
    Text::from_iter(lines)
}

/// Add hyperlink format to the buffer of notice
pub fn style_notice_hyperlinks(
    notice: &Notice,
    buffer: &mut Buffer,
    offset: Coordinate,
    hyperlink_style: Style,
) {
    if let Ok(notice_hyperlink_map) = NoticeHyperlinkMap::create(&notice) {
        for (mut y, row) in notice_hyperlink_map.get_map().iter().enumerate() {
            y = y + offset.y as usize + 1; // + 1 as the buffer will have a border
            for (mut x, char) in row.iter().enumerate() {
                x = x + offset.x as usize + 1; // as the buffer will have a border
                if char.is_some() {
                    if let Some(cell) = buffer.cell_mut((x as u16, y as u16)) {
                        cell.set_style(hyperlink_style);
                    }
                }
            }
        }
    }
}

/// Add notice hyperlinks to buffer of bored
pub fn style_bored_hyperlinks(bored: &Bored, buffer: &mut Buffer, hyperlink_style: Style) {
    if let Ok(bored_hyperlink_map) = BoredHyperlinkMap::create(&bored) {
        for (y, row) in bored_hyperlink_map.get_map().iter().enumerate() {
            // y += 1;
            for (x, char) in row.iter().enumerate() {
                // x += 1;
                if char.is_some() {
                    if let Some(cell) = buffer.cell_mut((x as u16, y as u16)) {
                        cell.set_style(hyperlink_style);
                    }
                }
            }
        }
    }
}

#[cfg(test)]

mod tests {

    use bored::notice::Notice;

    use crate::app::SurfBoredError;

    use super::*;

    #[test]
    fn test_bored_of_rects() -> Result<(), BoredError> {
        let mut bored = Bored::create("Hello", Coordinate { x: 120, y: 40 });
        let bored_of_rects = BoredOfRects::create(&bored, 0);
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
        // let hyperlink_style = Style::new().underlined();
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
        let theme = Theme::default();
        let hyperlink_style = Style::new().underlined();
        let mut bored = Bored::create("Hello", Coordinate { x: 60, y: 20 });
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write(
            "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
        )?;
        bored.add(notice, Coordinate { x: 5, y: 3 })?;
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write("world")?;
        bored.add(notice, Coordinate { x: 30, y: 10 })?;
        let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
        let mut buffer = Buffer::empty(bored_rect);
        let display_bored = DisplayBored::create(&bored, theme.clone());
        display_bored.render(bored_rect, &mut buffer);
        eprintln!("{:?}", buffer);
        let expected_output = r#"Buffer {
    area: Rect { x: 0, y: 0, width: 60, height: 20 },
    content: [
        "╭──────────────────────────────────────────────────────────╮",
        "│                                                          │",
        "│                                                          │",
        "│    ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓                        │",
        "│    ┃We are link bored.          ┃                        │",
        "│    ┃You are link bored.         ┃                        │",
        "│    ┃I am boooo                  ┃                        │",
        "│    ┃ooored.                     ┃                        │",
        "│    ┃Hello                       ┃                        │",
        "│    ┃World                       ┃                        │",
        "│    ┃                        ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓",
        "│    ┗━━━━━━━━━━━━━━━━━━━━━━━━┃world                       ┃",
        "│                             ┃                            ┃",
        "│                             ┃                            ┃",
        "│                             ┃                            ┃",
        "│                             ┃                            ┃",
        "│                             ┃                            ┃",
        "│                             ┃                            ┃",
        "│                             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛",
        "╰──────────────────────────────────────────────────────────╯",
    ],
    styles: [
        x: 0, y: 0, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 13, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 17, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 18, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 23, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 14, y: 5, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 18, y: 5, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 11, y: 6, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 16, y: 6, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 6, y: 7, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 12, y: 7, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
    ]
}"#;
        assert_eq!(expected_output, format!("{:?}", buffer));
        // just test view port with 100% view so should be the same as above
        let mut bored_view_port = BoredViewPort::create(&bored, Coordinate { x: 60, y: 20 });
        let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
        buffer = Buffer::empty(bored_rect);
        bored_view_port.render_view(&mut buffer, theme.clone());
        assert_eq!(expected_output, format!("{:?}", buffer));
        let mut bored_view_port = BoredViewPort::create(&bored, Coordinate { x: 40, y: 15 });
        bored_view_port.move_view(Coordinate { x: 5, y: 5 });
        let bored_rect = Rect::new(5, 5, 40, 15);
        buffer = Buffer::empty(bored_rect);
        let expected_output = r#"Buffer {
    area: Rect { x: 5, y: 5, width: 40, height: 15 },
    content: [
        "┃You are link bored.         ┃          ",
        "┃I am boooo                  ┃          ",
        "┃ooored.                     ┃          ",
        "┃Hello                       ┃          ",
        "┃World                       ┃          ",
        "┃                        ┏━━━━━━━━━━━━━━",
        "┗━━━━━━━━━━━━━━━━━━━━━━━━┃world         ",
        "                         ┃              ",
        "                         ┃              ",
        "                         ┃              ",
        "                         ┃              ",
        "                         ┃              ",
        "                         ┃              ",
        "                         ┗━━━━━━━━━━━━━━",
        "────────────────────────────────────────",
    ],
    styles: [
        x: 0, y: 0, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 9, y: 0, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 13, y: 0, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 6, y: 1, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 11, y: 1, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 1, y: 2, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 7, y: 2, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
    ]
}"#;
        bored_view_port.render_view(&mut buffer, theme.clone());
        eprintln!("{:?}", buffer);
        assert_eq!(expected_output, format!("{:?}", buffer));
        Ok(())
    }

    #[test]
    fn text_charcter_wrap() {
        let display_text = "I am so boored\nof\nthis really long \nline";
        let text = character_wrap(display_text.to_string(), 5);
        let expected_output = r#"I am 
so bo
ored
of
this 
reall
y lon
g 
line"#;
        assert_eq!(expected_output, format!("{}", text));
        eprintln!("\n{}", text);
    }

    #[test]
    fn test_style_notice_hyperlinks() -> Result<(), SurfBoredError> {
        let hyperlink_style = Style::new().underlined();
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write(
            "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
        )?;
        let notice_dimension = notice.get_dimensions();
        let display = notice.get_display().unwrap();
        let display_text = display.get_display_text();
        let display_text = character_wrap(display_text, notice.get_text_width());
        let notice_rect = Rect::new(0, 0, notice_dimension.x, notice_dimension.y);
        let notice_block = Block::default().borders(Borders::ALL);
        let notice_text = Paragraph::new(display_text).block(notice_block);
        let mut notice_buffer = Buffer::empty(notice_rect);
        notice_text.render(notice_rect, &mut notice_buffer);
        style_notice_hyperlinks(
            &notice,
            &mut notice_buffer,
            Coordinate { x: 0, y: 0 },
            hyperlink_style,
        );
        let expected_output = r#"Buffer {
    area: Rect { x: 0, y: 0, width: 30, height: 9 },
    content: [
        "┌────────────────────────────┐",
        "│We are link bored.          │",
        "│You are link bored.         │",
        "│I am boooo                  │",
        "│ooored.                     │",
        "│Hello                       │",
        "│World                       │",
        "│                            │",
        "└────────────────────────────┘",
    ],
    styles: [
        x: 0, y: 0, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 8, y: 1, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 12, y: 1, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 13, y: 1, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 18, y: 1, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 9, y: 2, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 13, y: 2, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 6, y: 3, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 11, y: 3, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 1, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 7, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
    ]
}"#;
        assert_eq!(expected_output, format!("{:?}", notice_buffer));
        eprintln!("{:?}", notice_buffer);
        Ok(())
    }

    #[test]
    fn test_style_bored_hyperlinks() -> Result<(), SurfBoredError> {
        let theme = Theme::default();
        let mut bored = Bored::create("Hello", Coordinate { x: 40, y: 20 });
        let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write(
                "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
            )?;
        bored.add(notice, Coordinate { x: 5, y: 3 })?;
        let mut notice = Notice::create(Coordinate { x: 10, y: 13 });
        notice.write(
                "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
            )?;
        bored.add(notice, Coordinate { x: 10, y: 5 })?;
        let mut notice = Notice::create(Coordinate { x: 10, y: 13 });
        notice.write(
                "We are [link](url) [bored](url).\nYou are [link](url) bored.\nI am [boooo\nooored](url).\nHello\nWorld",
            )?;
        bored.add(notice, Coordinate { x: 14, y: 7 })?;
        let mut bored_buffer = Buffer::empty(bored_rect);
        let display_bored = DisplayBored::create(&bored, theme.clone());
        display_bored.render(bored_rect, &mut bored_buffer);
        eprintln!("{}", format!("{:?}", bored_buffer));
        let expected_output = r#"Buffer {
    area: Rect { x: 0, y: 0, width: 40, height: 20 },
    content: [
        "╭──────────────────────────────────────╮",
        "│                                      │",
        "│                                      │",
        "│    ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓    │",
        "│    ┃We are link bored.          ┃    │",
        "│    ┃You ┏━━━━━━━━┓ored.         ┃    │",
        "│    ┃I am┃We are l┃              ┃    │",
        "│    ┃ooor┃ink┏━━━━━━━━┓          ┃    │",
        "│    ┃Hell┃d. ┃We are l┃          ┃    │",
        "│    ┃Worl┃You┃ink bore┃          ┃    │",
        "│    ┃    ┃lin┃d.      ┃          ┃    │",
        "│    ┗━━━━┃ed.┃You are ┃━━━━━━━━━━┛    │",
        "│         ┃I a┃link bor┃               │",
        "│         ┃oo ┃ed.     ┃               │",
        "│         ┃ooo┃I am boo┃               │",
        "│         ┃Hel┃oo      ┃               │",
        "│         ┃Wor┃ooored. ┃               │",
        "│         ┗━━━┃Hello   ┃               │",
        "│             ┃World   ┃               │",
        "╰─────────────┗━━━━━━━━┛───────────────╯",
    ],
    styles: [
        x: 0, y: 0, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 13, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 17, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 18, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 23, y: 4, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 18, y: 6, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 19, y: 6, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 6, y: 7, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 10, y: 7, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 11, y: 7, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 14, y: 7, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 11, y: 8, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 12, y: 8, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 22, y: 8, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 23, y: 8, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 15, y: 9, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 18, y: 9, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 19, y: 9, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 23, y: 9, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 11, y: 10, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 14, y: 10, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 15, y: 10, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 16, y: 10, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 15, y: 12, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 19, y: 12, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 11, y: 13, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 13, y: 13, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 11, y: 14, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 14, y: 14, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 20, y: 14, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 23, y: 14, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 15, y: 15, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 17, y: 15, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
        x: 15, y: 16, fg: Reset, bg: Reset, underline: Reset, modifier: UNDERLINED,
        x: 21, y: 16, fg: Reset, bg: Reset, underline: Reset, modifier: NONE,
    ]
}"#;
        assert_eq!(expected_output, format!("{:?}", bored_buffer));
        Ok(())
    }
}
