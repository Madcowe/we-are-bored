/*
Copyright (C) 2025 We are bored

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::{self, Display, Notice, NoticeHyperlinkMap, get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, BoredHyperlinkMap, Coordinate};
use core::panic;
use ratatui::buffer::Buffer;
use ratatui::style::{Styled, Stylize};
use ratatui::symbols::border;
use ratatui::widgets::{BorderType, Widget};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    buffer::Cell,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::cmp::{max, min};
use std::time::Duration;
use tokio::task;
use tokio::time::sleep;

use crate::app::{App, CreateMode, DraftMode, SurfBoredError, View};
use crate::display_bored::{BoredViewPort, DisplayBored};
use crate::display_bored::{character_wrap, style_notice_hyperlinks};
use crate::theme::Theme;

pub fn ui(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let mut bored_name = String::new();
    let mut bored_url = String::new();
    let mut status_text = String::new();
    // format!(
    // "Current: {:?} previous: {:?} interuppted: {:?} {}",
    // app.current_view, app.previous_view, app.interupted_view, app.status
    // );
    //"Connected, no bored loaded";
    // let mut status_text = "Connected, no bored loaded";
    let ui_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Fill(1),
            Constraint::Length(5),
        ])
        .split(area);
    let bored = app.get_current_bored();
    if let Some(bored) = bored {
        bored_url = format!(
            "{}",
            app.client.as_ref().unwrap().get_bored_address().unwrap()
        );
        bored_name = bored.get_name().to_owned() + "\n";
        let mut bored_view_port = BoredViewPort::create(
            &bored,
            Coordinate {
                x: ui_chunks[1].width,
                y: ui_chunks[1].height,
            },
            app.selected_notice,
        );
        if let View::NoticeView {
            hyperlinks_index: _,
        } = app.current_view
        {
        } else {
            if let Some(view_top_left) = app.bored_view_port.as_ref().map(|s| s.get_view_top_left())
            {
                bored_view_port.move_view(view_top_left);
            }
            let mut bored_view_buffer = Buffer::empty(ui_chunks[1]);
            bored_view_port.render_view(&mut bored_view_buffer, app.theme.clone());
            frame.buffer_mut().merge(&bored_view_buffer);
        }
        app.bored_view_port = Some(bored_view_port);
    } else {
        let view_port_block = Block::default().style(app.theme.text_style());
        frame.render_widget(view_port_block, ui_chunks[1]);
    }
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::QuadrantOutside)
        .style(app.theme.header_style())
        .bold();
    let mut url_style = app.theme.header_style();
    if app.current_view == View::GoToView {
        bored_url = app.goto_input.clone();
        if bored_url.len() < 72 {
            bored_url = bored_url.clone() + &str::repeat(" ", 72 - bored_url.len());
        }
        url_style = app.theme.text_style();
    }
    let name_span = Span::styled(bored_name, app.theme.header_style());
    let url_span = Span::styled(bored_url, url_style);
    let title_text = Text::from_iter(vec![name_span, url_span]);
    // let title_text = bored_name + "\n" + &bored_url;
    let title = Paragraph::new(title_text).block(title_block);
    frame.render_widget(title, ui_chunks[0]);

    // modify based on current_view
    match &app.current_view {
        View::ErrorView(e) => {
            status_text = "Press (enter) to contunue or (q) to quit".to_string();
            let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4)); //centered_rect(60, 60, area);
            let navigation_text = "Press (enter) to contiune or (q) to quit.";
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
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
                    status_text =
                        "Type to enter bored name, press (enter) to proceed or (esc) to go leave"
                            .to_string();
                    name_block = name_block.clone().style(app.theme.inverted_text_style())
                }
                CreateMode::PrivateKey => {
                    status_text = "Type to enter key (or use terminal emulator paste) (enter) to proceed, (tab) to edit name or (esc) to leave".to_string();
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
                let bored = app
                    .get_current_bored()
                    .expect("There should not be a draft without a bored");
                match draft_mode {
                    DraftMode::Content => {
                        status_text = "Type to enter message, (ctrl + p) to position notice or (esc) to leave".to_string();
                        let display = draft.get_display().unwrap();
                        let display_text = display.get_display_text();
                        let display_text = character_wrap(display_text, draft.get_text_width());
                        let draft_rect = get_draft_postion_on_viewport(
                            &draft,
                            &app.bored_view_port,
                            ui_chunks[0].height,
                        );
                        let draft_block = Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick)
                            .style(app.theme.text_style());
                        let draft_text = Paragraph::new(display_text).block(draft_block);
                        let mut draft_buffer = Buffer::empty(draft_rect);
                        draft_text.render(draft_rect, &mut draft_buffer);
                        // render hyperlinks
                        style_notice_hyperlinks(
                            &draft,
                            &mut draft_buffer,
                            Coordinate {
                                x: draft_rect.x,
                                y: draft_rect.y,
                            },
                            app.theme.hyperlink_style(),
                        );
                        frame.buffer_mut().merge(&draft_buffer);
                    }
                    DraftMode::Position => {
                        status_text = "Use (the arrow keys) to postion the notice and (enter) to place or (esc) to edit text".to_string();
                        let display = draft.get_display().unwrap();
                        let display_text = display.get_display_text();
                        let display_text = character_wrap(display_text, draft.get_text_width());
                        let draft_rect = get_draft_postion_on_viewport(
                            &draft,
                            &app.bored_view_port,
                            ui_chunks[0].height,
                        );
                        let draft_block = Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick)
                            .style(app.theme.text_style());
                        let draft_text = Paragraph::new(display_text).block(draft_block);
                        let mut draft_buffer = Buffer::empty(draft_rect);
                        draft_text.render(draft_rect, &mut draft_buffer);
                        style_notice_hyperlinks(
                            &draft,
                            &mut draft_buffer,
                            Coordinate {
                                x: draft_rect.x,
                                y: draft_rect.y,
                            },
                            app.theme.hyperlink_style(),
                        );
                        frame.buffer_mut().merge(&draft_buffer);
                    }
                    _ => (),
                }
            }
        }
        View::BoredView => {
            status_text = "Use (the arrow keys) to select a notice in that direction, (tab) to cycle selection, (enter) to view notice, (n) to create new notice, (g) to goto url (c) to create new bored or (q) to quit".to_string();
        }
        View::NoticeView { hyperlinks_index } => {
            if let Some(notice) = app.get_selected_notice() {
                status_text = "Press (tab) to cycle through hyperlinks, (enter) to activte selected hyperlink and (esc) to leave".to_string();
                let pop_up_rect = area.inner(Margin::new(
                    safe_subtract_u16(area.width, notice.get_dimensions().x) / 2,
                    safe_subtract_u16(area.height, notice.get_dimensions().y) / 2,
                ));
                Clear.render(pop_up_rect, frame.buffer_mut());
                let display = get_display(
                    notice.get_content(),
                    get_hyperlinks(notice.get_content()).unwrap_or(vec![]),
                );
                let pop_up_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::QuadrantOutside)
                    .style(app.theme.inverted_text_style());
                let pop_up_text =
                    character_wrap(display.get_display_text(), notice.get_text_width());
                let pop_up_paragraph =
                    Paragraph::new(pop_up_text.clone()).block(pop_up_block.clone());
                let mut pop_up_buffer = Buffer::empty(pop_up_rect);
                pop_up_paragraph.render(pop_up_rect, &mut pop_up_buffer);
                // render hyperlinks
                style_notice_hyperlinks(
                    &notice,
                    &mut pop_up_buffer,
                    Coordinate {
                        x: pop_up_rect.x,
                        y: pop_up_rect.y,
                    },
                    app.theme.hyperlink_style(),
                );
                // Highlight selected hyperlink
                if let Ok(notice_hyperlink_map) = NoticeHyperlinkMap::create(&notice) {
                    for (mut y, row) in notice_hyperlink_map.get_map().iter().enumerate() {
                        y = y + pop_up_rect.y as usize + 1; // + 1 as the buffer will have a border
                        for (mut x, index) in row.iter().enumerate() {
                            x = x + pop_up_rect.x as usize + 1; // as the buffer will have a border
                            if index == hyperlinks_index && index.is_some() {
                                if let Some(cell) = pop_up_buffer.cell_mut((x as u16, y as u16)) {
                                    cell.set_style(app.theme.text_style());
                                }
                            }
                        }
                    }
                }
                frame.buffer_mut().merge(&pop_up_buffer);
            }
        }
        View::GoToView => {
            status_text = "Type to enter URL (or use terminal emulator paste) (enter) to go to address (esc) to leave".to_string();
        }
        _ => (),
    }
    // setup status area
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::QuadrantOutside)
        .style(app.theme.header_style())
        .bold();
    // let status_rect = Rect::new(0, area.height - 5, area.width, 5);
    // status_text = format!("{:?}\n{}", app.status, status_text);
    let status = Paragraph::new(Text::styled(status_text, Style::default()))
        .wrap(Wrap { trim: false })
        .block(status_block);
    frame.render_widget(status, ui_chunks[2]);
    // status.render(status_rect, frame.buffer_mut());
}

fn get_draft_postion_on_viewport(
    draft: &Notice,
    bored_view_port: &Option<BoredViewPort>,
    y_offset: u16,
) -> Rect {
    let view_top_left = match bored_view_port {
        Some(bored_view_port) => bored_view_port.get_view_top_left(),
        None => Coordinate { x: 0, y: 0 },
    };
    let x = safe_subtract_u16(draft.get_top_left().x, view_top_left.x);
    let y = safe_subtract_u16(draft.get_top_left().y, view_top_left.y) + y_offset;
    Rect::new(x, y, draft.get_dimensions().x, draft.get_dimensions().y)
}

/// Returns 0 if subraction overflow
pub fn safe_subtract_u16(a: u16, b: u16) -> u16 {
    if (a as i32 - b as i32) < 0 { 0 } else { a - b }
}

/// pops up a wating popup while awaiting a future
pub async fn wait_pop_up<B: Backend>(
    // frame: &mut Frame<'_>,
    terminal: &mut Terminal<B>,
    previous_buffer: Buffer,
    future: impl Future<Output = Result<(), SurfBoredError>>,
    message: &str,
    theme: Theme,
) -> Result<(), SurfBoredError> {
    let mut count = 0;
    let animate = async {
        let mut antimation = Antimation::new();
        while count < 1200 {
            let result = terminal.draw(|frame| {
                frame.buffer_mut().merge(&previous_buffer);
                let area = frame.area();
                let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4));
                Clear.render(pop_up_rect, frame.buffer_mut());
                let pop_up_block = Block::default()
                    .title("Working...")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .style(theme.header_style());
                let ant_frame = antimation.next_frame();
                let pop_up_text = Paragraph::new(Text::styled(
                    format!("{message}\n {ant_frame}"),
                    Style::default(),
                ))
                .block(pop_up_block);
                frame.render_widget(pop_up_text, pop_up_rect);
            });
            count += 1;
            sleep(Duration::from_millis(500)).await;
            match result {
                Err(_) => return Err::<(), SurfBoredError>(SurfBoredError::CannotRenderWait),
                _ => (),
            }
        }
        Err(SurfBoredError::StillWaiting)
    };
    tokio::select! {
        e = animate => { e? }
        f = future => { f? }
    }
    Ok(())
}

pub struct Antimation {
    count: usize,
}
impl Antimation {
    fn new() -> Antimation {
        Antimation { count: 0 }
    }

    fn next_frame(&mut self) -> String {
        let frame = if self.count == 0 {
            "o o    \n  \\\\\n  (\"\")\n  >||<\n   /\\".to_string()
        } else if self.count == 2 {
            "   o o\n    //\n  (\"\")\n  >||<\n   /\\".to_string()
        } else {
            "  oo  \n   ||  \n  (\'\')\n  >||<  \n   /\\  ".to_string()
        };
        if self.count >= 3 {
            self.count = 0
        } else {
            self.count += 1;
        }
        frame
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn test_safe_subtract_u16() {
        assert_eq!(safe_subtract_u16(3, 2), 1);
        assert_eq!(safe_subtract_u16(3, 3), 0);
        assert_eq!(safe_subtract_u16(3, 4), 0);
    }

    #[test]
    fn test_get_draft_notice_on_viewport() {
        let bored = Bored::create("Test", Coordinate { x: 120, y: 40 });
        let draft = Notice::create(Coordinate { x: 30, y: 10 });
        let draft_postion_on_viewport = get_draft_postion_on_viewport(&draft, &None, 4);
        assert_eq!(draft_postion_on_viewport, Rect::new(0, 4, 30, 10));
        let mut bored_view_port = BoredViewPort::create(&bored, Coordinate { x: 40, y: 15 }, None);
        bored_view_port.move_view(Coordinate { x: 80, y: 5 });
        let draft_postion_on_viewport =
            get_draft_postion_on_viewport(&draft, &Some(bored_view_port), 4);
        assert_eq!(draft_postion_on_viewport, Rect::new(0, 4, 30, 10));
        let mut bored_view_port = BoredViewPort::create(&bored, Coordinate { x: 40, y: 15 }, None);
        bored_view_port.move_view(Coordinate { x: 10, y: 5 });
        let draft_postion_on_viewport =
            get_draft_postion_on_viewport(&draft, &Some(bored_view_port), 4);
        assert_eq!(draft_postion_on_viewport, Rect::new(0, 4, 30, 10));
    }
}
