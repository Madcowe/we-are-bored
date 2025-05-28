use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::{Display, Notice, NoticeHyperlinkMap, get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, Coordinate};
use rand::rand_core::block::BlockRng;
use ratatui::buffer::Buffer;
use ratatui::prelude::BlockExt;
use ratatui::style::{Styled, Stylize};
use ratatui::widgets::{BorderType, Widget};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::fmt::Pointer;
use std::ops::Deref;
use std::str::FromStr;

use crate::app::{App, CreateMode, DraftMode, GoToMode, HyperlinkMode, View};
use crate::display_bored::DisplayBored;
use crate::display_bored::{character_wrap, render_hyperlinks, style_notice_hyperlinks};

pub fn ui(frame: &mut Frame, app: &mut App) {
    // setup base interfact
    let area = frame.area();
    let mut title_text = String::new();
    let mut status_text = format!(
        "Current: {:?} previous: {:?} key: {}",
        app.current_view, app.previous_view, app.status
    ); //"Connected, no bored loaded";
    // let mut status_text = "Connected, no bored loaded";
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
        .border_type(BorderType::QuadrantOutside)
        .style(app.theme.header_style())
        .bold();
    let title_rect = Rect::new(0, 0, area.width, 4);
    let title = Paragraph::new(Text::raw(title_text)).block(title_block);
    frame.render_widget(title, title_rect);
    let bored_view_block = Block::default().bg(Color::Black);
    let bored_view_rect = Rect::new(0, 4, area.width, area.height - 7);
    frame.render_widget(bored_view_block, bored_view_rect);
    let status_block = Block::default()
        .borders(Borders::ALL)
        .bold()
        .border_type(BorderType::QuadrantOutside)
        .style(app.theme.header_style())
        .bold();
    let status_rect = Rect::new(0, area.height - 27, area.width, 27);
    let status = Paragraph::new(Text::styled(status_text, Style::default()))
        .wrap(Wrap { trim: false })
        .block(status_block);
    status.render(status_rect, frame.buffer_mut());

    // modify based on current_view
    match &app.current_view {
        View::ErrorView(e) => {
            let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4)); //centered_rect(60, 60, area);
            let navigation_text = "Press (enter) to contiune or (q) to quit.";
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
                .title("Error")
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .style(app.theme.text_style());
            frame.render_widget(pop_up_block, pop_up_rect);
            let pop_up_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(100),
                    Constraint::Min(navigation_text.lines().count() as u16),
                ])
                .split(pop_up_rect);
            let pop_up_text = Paragraph::new(Text::styled(format!("{e}"), Style::default()))
                .wrap(Wrap { trim: false });
            frame.render_widget(pop_up_text, pop_up_chunks[0]);
            let navigation_text =
                Paragraph::new(Text::styled(navigation_text, Style::default()).not_rapid_blink())
                    .alignment(Alignment::Center);
            frame.render_widget(navigation_text, pop_up_chunks[1]);
        }
        View::CreateView(create_mode) => {
            let pop_up_rect = area.inner(Margin::new(area.width / 8, area.height / 5));
            let navigation_text =
                "Press (tab) to toggle input, (Y) to paste from system clipboard (esc) to cancel";
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
                .title("Enter bored name and private key of funding wallet*")
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .style(app.theme.text_style());
            // .bg(Color::Black);
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
            let mut name_block = Block::default().title("Name").style(app.theme.text_style());
            let mut key_block = Block::default()
                .title("Private key of funding wallet")
                .style(app.theme.text_style());
            match create_mode {
                CreateMode::Name => {
                    name_block = name_block.clone().style(app.theme.inverted_text_style())
                }
                CreateMode::PrivateKey => {
                    key_block = key_block.clone().style(app.theme.inverted_text_style())
                }
            };
            let name_text = Paragraph::new(app.name_input.clone()).block(name_block);
            let key_text = Paragraph::new(app.key_input.clone()).block(key_block);
            frame.render_widget(name_text, pop_up_chunks[0]);
            frame.render_widget(key_text, pop_up_chunks[1]);
        }
        View::DraftView(draft_mode) => {
            if let Some(draft) = app.get_draft() {
                match draft_mode {
                    DraftMode::Content => {
                        let draft_dimension = draft.get_dimensions();
                        let display = draft.get_display().unwrap();
                        let display_text = display.get_display_text();
                        let display_text = character_wrap(display_text, draft.get_text_width());
                        // app.status = format!("{:?}", display_text);
                        // position so aprox in center of frame
                        let x = (area.width - draft_dimension.x) / 2;
                        let y = (area.height - draft_dimension.y) / 2;
                        let draft_rect = Rect::new(x, y, draft_dimension.x, draft_dimension.y);
                        let mut draft_block = Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick)
                            .style(app.theme.text_style());
                        let draft_text = Paragraph::new(display_text).block(draft_block);
                        let mut draft_buffer = Buffer::empty(draft_rect);
                        draft_text.render(draft_rect, &mut draft_buffer);
                        app.status = format!("{:?}", draft_buffer);
                        // render hyperlinks
                        style_notice_hyperlinks(
                            &draft,
                            &mut draft_buffer,
                            Coordinate { x, y },
                            app.theme.hyperlink_style(),
                        );
                        app.status = app.status.clone() + &format!("{:?}", draft_buffer);
                        frame.buffer_mut().merge(&draft_buffer);
                    }
                    _ => (),
                }
            }
        }

        _ => (),
    }
}
