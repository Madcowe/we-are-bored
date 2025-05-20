use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::{Notice, get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, Coordinate};
use ratatui::buffer::Buffer;
use ratatui::prelude::BlockExt;
use ratatui::style::{Styled, Stylize};
use ratatui::widgets::Widget;
use ratatui::{
    Frame,
    crossterm::style::StyledContent,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::ops::Deref;
use std::str::FromStr;

use crate::app::{App, CreateMode, DraftMode, GoToMode, HyperlinkMode, View};
use crate::display_bored::DisplayBored;

pub fn ui(frame: &mut Frame, app: &App) {
    // setup base interfact
    let area = frame.area();
    let mut title_text = String::new();
    let mut status_text = "Connected, no bored loaded";
    let bored = app.get_current_bored();
    if let Some(bored) = bored {
        let bored_name = format!(
            "{}",
            app.client.as_ref().unwrap().get_bored_address().unwrap()
        );
        title_text = bored.get_name().to_owned() + "\n" + &bored_name;
        status_text = "Connected, bored loded";
    }
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    let title_rect = Rect::new(0, 0, area.width, 4);
    let title = Paragraph::new(Text::styled(
        title_text,
        Style::default(), //.fg(Color::from_str("#529B81").unwrap()),
    ))
    .block(title_block);
    frame.render_widget(title, title_rect);
    let bored_view_block = Block::default().bg(Color::Black);
    let bored_view_rect = Rect::new(0, 4, area.width, area.height - 7);
    frame.render_widget(bored_view_block, bored_view_rect);
    let status_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    let status_rect = Rect::new(0, area.height - 3, area.width, 3);
    let status = Paragraph::new(Text::styled(status_text, Style::default())).block(status_block);
    status.render(status_rect, frame.buffer_mut());

    // modify based on current_view
    match &app.current_view {
        View::ErrorView(e) => {
            let pop_up_rect = centered_rect(60, 60, area);
            create_pop_up(
                frame,
                pop_up_rect,
                &format!("{e}"),
                "Press (enter) to contiune or (q) to quit.",
            );
        }
        _ => (),
    }
}

/// function to generate pop ups windows
fn create_pop_up(frame: &mut Frame, pop_up_rect: Rect, content: &str, navigation_text: &str) {
    Clear.render(pop_up_rect, frame.buffer_mut());
    let pop_up_block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .style(Style::default());
    frame.render_widget(pop_up_block, pop_up_rect);
    let pop_up_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(100),
            Constraint::Min(navigation_text.lines().count() as u16),
        ])
        .split(pop_up_rect);
    let pop_up_text = Paragraph::new(Text::styled(format!("{content}"), Style::default()));
    frame.render_widget(pop_up_text, pop_up_chunks[0]);
    let navigation_text =
        Paragraph::new(Text::styled(navigation_text, Style::default()).not_rapid_blink())
            .alignment(Alignment::Center);
    frame.render_widget(navigation_text, pop_up_chunks[1]);
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
