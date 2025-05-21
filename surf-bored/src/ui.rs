use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::{Notice, get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, Coordinate};
use ratatui::buffer::Buffer;
use ratatui::prelude::BlockExt;
use ratatui::style::{Styled, Stylize};
use ratatui::widgets::Widget;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
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
    let mut status_text = format!("{:?}", app.current_view); //"Connected, no bored loaded";
    let bored = app.get_current_bored();
    if let Some(bored) = bored {
        let bored_name = format!(
            "{}",
            app.client.as_ref().unwrap().get_bored_address().unwrap()
        );
        title_text = bored.get_name().to_owned() + "\n" + &bored_name;
        // status_text = "Connected, bored loded";
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
            let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4)); //centered_rect(60, 60, area);
            create_error_pop_up(
                frame,
                pop_up_rect,
                &format!("{e}"),
                "Press (enter) to contiune or (q) to quit.",
            );
        }
        View::CreateView(create_mode) => {
            let pop_up_rect = area.inner(Margin::new(area.width / 8, area.height / 5));
            // create_create_pop_up(frame, pop_up_rect, create_mode);
            let navigation_text =
                "Press (tab) to toggle input, (Y) to paste from system clipboard (esc) to cancel";
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
                .title("Enter bored name and private key of funding wallet*")
                .borders(Borders::ALL)
                .style(app.theme.style())
                .bg(Color::Black);
            frame.render_widget(pop_up_block, pop_up_rect);
            let pop_up_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                    Constraint::Min(navigation_text.lines().count() as u16),
                ])
                .split(pop_up_rect);
            let mut name_block = Block::default()
                .title("Name")
                .borders(Borders::ALL)
                .style(app.theme.style());
            let mut key_block = Block::default()
                .title("Private key of funding wallet")
                .borders(Borders::ALL)
                .style(app.theme.style());
            match create_mode {
                CreateMode::Name => {
                    name_block = name_block.clone().style(app.theme.inverted_text_style())
                }
                CreateMode::PrivateKey => {
                    key_block = key_block.clone().style(app.theme.inverted_text_style())
                }
            };
            frame.render_widget(name_block, pop_up_chunks[0]);
            frame.render_widget(key_block, pop_up_chunks[1]);
        }

        _ => (),
    }
}

/// function to invert fore and background colours for highlighting
// problem style don't have fg and bg unless set???
fn invert_colours(styled: &mut (impl Styled + Clone)) {
    let fg = styled.style().bg.unwrap();
    let bg = styled.style().fg.unwrap();
    styled.clone().set_style(styled.style().fg(fg).bg(bg));
}

/// function to generate error pop ups windows
fn create_error_pop_up(frame: &mut Frame, pop_up_rect: Rect, content: &str, navigation_text: &str) {
    Clear.render(pop_up_rect, frame.buffer_mut());
    let pop_up_block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .style(Style::default())
        .bg(Color::Black);
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

/// function to genrate create bored popup
fn create_create_pop_up(frame: &mut Frame, pop_up_rect: Rect, create_mode: &CreateMode) {}
