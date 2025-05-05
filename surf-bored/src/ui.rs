use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::{get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, Coordinate};
use ratatui::prelude::BlockExt;
use ratatui::style::Styled;
use ratatui::{
    Frame,
    crossterm::style::StyledContent,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::str::FromStr;

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

    /// returns a vector of blocks with the notice text ect attached to the rects
    fn notices_to_blocks(&self, bored: &Bored) -> Result<Vec<Block>, BoredError> {
        let mut blocks = vec![];
        let notices = bored.get_notices().into_iter().zip(self.notices.clone());
        for (notice, notice_rect) in notices {
            let display = get_display(notice.get_content(), get_hyperlinks(notice.get_content())?);
            let block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default());
            let paragraph = Paragraph::new(Text::styled(
                display.get_display_text(),
                Style::default().fg(Color::from_str("#529B81").unwrap()),
            ))
            .block(block.clone());
            blocks.push(block);
        }
        Ok(blocks)
    }
}

pub fn ui(frame: &mut Frame, app: &App) {
    // let chunks = Layout::default()
    //     .direction(Direction::Vertical)
    //     .constraints([
    //         Constraint::Length(4),
    //         Constraint::Min(1),
    //         Constraint::Length(3),
    //     ])
    //     .split(frame.area());

    // let mut bored_name = "Not connected";
    // let mut bored_address = "".to_string();
    // if let Some(client) = &app.client {
    //     bored_name = client.get_bored_name().unwrap_or("Test bored");
    //     bored_address = match client.get_bored_address() {
    //         Ok(bored_address) => format!("{}", bored_address),
    //         Err(_) => "".to_string(),
    //     };
    // }
    // let title = Paragraph::new(Text::styled(
    //     bored_name.to_string() + "\n" + &bored_address,
    //     Style::default().fg(Color::from_str("#529B81").unwrap()),
    // ))
    // .block(title_block);

    let bored = app.get_current_bored();
    if bored.is_none() {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        let title_rect = Rect::new(0, 0, 120, 4);
        let title = Paragraph::new(Text::styled(
            "Test bored",
            Style::default(), //.fg(Color::from_str("#529B81").unwrap()),
        ))
        .block(title_block);
        frame.render_widget(title, title_rect);
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
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
        let blocks = bored_of_rects.notices_to_blocks(&bored)?;
        assert!(blocks.is_empty());
        let notice = Notice::create(Coordinate { x: 60, y: 18 });
        bored.add(notice, Coordinate { x: 10, y: 5 })?;
        let bored_of_rects = BoredOfRects::create(&bored, 0);
        let blocks = bored_of_rects.notices_to_blocks(&bored)?;
        assert_eq!(blocks.len(), 1);
        Ok(())
    }
}
