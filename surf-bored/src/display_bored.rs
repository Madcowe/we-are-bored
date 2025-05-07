use bored::notice::{get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, Coordinate};
use ratatui::buffer::Buffer;
use ratatui::style::{Styled, Stylize};
use ratatui::{
    Frame,
    crossterm::style::StyledContent,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget, Wrap},
};

use crate::app::{App, CreateMode, DraftMode, GoToMode, HyperlinkMode, View};

/// Represent the layout of the bored an it's notices in rects
struct BoredOfRects {
    bored: Rect,
    notices: Vec<Rect>,
}

impl BoredOfRects {
    fn create(bored: &Bored, y_offset: u16) -> BoredOfRects {
        let mut notices = vec![];
        for notice in bored.get_notices() {
            let notice_rect = Rect::new(
                notice.get_top_left().x,
                notice.get_top_left().y + y_offset,
                notice.get_dimensions().x,
                notice.get_dimensions().y + y_offset,
            );
            notices.push(notice_rect);
        }
        let bored_rect = Rect::new(
            0,
            y_offset,
            bored.get_dimensions().x,
            bored.get_dimensions().y + y_offset,
        );
        BoredOfRects {
            bored: bored_rect,
            notices,
        }
    }

    /// returns a vector of blocks with the notice text attached to the rects
    fn notices_to_blocks(&self, bored: &Bored) -> Result<Vec<(Paragraph, Rect)>, BoredError> {
        let mut display_notices = vec![];
        let notices = bored.get_notices().into_iter().zip(self.notices.clone());
        for (notice, notice_rect) in notices {
            let display = get_display(notice.get_content(), get_hyperlinks(notice.get_content())?);
            let block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default());
            // .bg(Color::Black);
            let paragraph = Paragraph::new(Text::styled(
                display.get_display_text(),
                Style::default(), //.fg(Color::from_str("#529B81").unwrap()),
            ))
            .block(block.clone());
            display_notices.push((paragraph, notice_rect));
        }
        Ok(display_notices)
    }
}

/// widget that can render the entirety of a bored
pub struct DisplayBored {
    bored: Bored,
    bored_rect: Rect,
}
impl Widget for DisplayBored {
    fn render(self, _: Rect, buffer: &mut Buffer) {
        let bored_of_rects = BoredOfRects::create(&self.bored, 0);
        if let Ok(display_notices) = bored_of_rects.notices_to_blocks(&self.bored) {
            for (display_notice, notice_rect) in display_notices {
                Clear.render(notice_rect, buffer);
                display_notice.render(notice_rect, buffer);
            }
        }
    }
}

impl DisplayBored {
    pub fn create(bored: &Bored) -> DisplayBored {
        let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
        DisplayBored {
            bored: bored.clone(),
            bored_rect,
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
        if view_top_left
            .add(&self.view_dimensions)
            .within(&self.bored_dimensions)
        {
            self.view_top_left = view_top_left;
        }
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
        assert_eq!(bored_of_rects.bored, Rect::new(0, 0, 120, 40));
        assert!(bored_of_rects.notices.is_empty());
        let notice = Notice::create(Coordinate { x: 60, y: 18 });
        bored.add(notice, Coordinate { x: 10, y: 5 })?;
        let bored_of_rects = BoredOfRects::create(&bored, 0);
        assert_eq!(bored_of_rects.notices[0].x, 10);
        assert_eq!(bored_of_rects.notices[0].y, 5);
        assert_eq!(bored_of_rects.notices[0].width, 60);
        assert_eq!(bored_of_rects.notices[0].height, 18);
        Ok(())
    }

    #[test]
    fn test_notices_to_blocks() -> Result<(), BoredError> {
        let mut bored = Bored::create("Hello", Coordinate { x: 120, y: 40 });
        let bored_of_rects = BoredOfRects::create(&bored, 0);
        let display_notices = bored_of_rects.notices_to_blocks(&bored)?;
        assert!(display_notices.is_empty());
        let notice = Notice::create(Coordinate { x: 60, y: 18 });
        bored.add(notice, Coordinate { x: 10, y: 5 })?;
        let bored_of_rects = BoredOfRects::create(&bored, 0);
        let display_notices = bored_of_rects.notices_to_blocks(&bored)?;
        assert_eq!(display_notices.len(), 1);
        Ok(())
    }

    #[test]
    fn test_display_bored_render() -> Result<(), BoredError> {
        let mut bored = Bored::create("Hello", Coordinate { x: 60, y: 20 });
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write("hello")?;
        bored.add(notice, Coordinate { x: 5, y: 3 })?;
        let mut notice = Notice::create(Coordinate { x: 30, y: 9 });
        notice.write("world")?;
        bored.add(notice, Coordinate { x: 30, y: 10 })?;
        let bored_rect = Rect::new(0, 0, bored.get_dimensions().x, bored.get_dimensions().y);
        let mut buffer = Buffer::empty(bored_rect);
        let display_bored = DisplayBored::create(&bored);
        display_bored.render(bored_rect, &mut buffer);
        let expected_output = r#"Buffer {
    area: Rect { x: 0, y: 0, width: 60, height: 20 },
    content: [
        "                                                            ",
        "                                                            ",
        "                                                            ",
        "     ┌────────────────────────────┐                         ",
        "     │hello                       │                         ",
        "     │                            │                         ",
        "     │                            │                         ",
        "     │                            │                         ",
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
    ]
}"#;
        eprintln!("{:?}", buffer);
        assert_eq!(expected_output, format!("{:?}", buffer));
        Ok(())
    }
}
