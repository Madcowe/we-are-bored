use bored::bored_client::{BoredClient, ConnectionType};
use bored::notice::{self, Display, Notice, NoticeHyperlinkMap, get_display, get_hyperlinks};
use bored::{Bored, BoredAddress, BoredError, Coordinate};
use ratatui::buffer::Buffer;
use ratatui::style::{Styled, Stylize};
use ratatui::widgets::{BorderType, Widget};
use ratatui::{
    Frame,
    buffer::Cell,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::cmp::{max, min};

use crate::app::{App, CreateMode, DraftMode, GoToMode, HyperlinkMode, View};
use crate::display_bored::{BoredViewPort, DisplayBored};
use crate::display_bored::{character_wrap, style_notice_hyperlinks};

pub fn ui(frame: &mut Frame, app: &mut App) {
    // setup base interface
    let area = frame.area();
    let mut title_text = String::new();
    let mut status_text = format!(
        "Current: {:?} previous: {:?} interupted: {:?} notice: {:?} {}",
        app.current_view, app.previous_view, app.interupted_view, app.selected_notice, app.status
    ); //"Connected, no bored loaded";
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
        let bored_name = format!(
            "{}",
            app.client.as_ref().unwrap().get_bored_address().unwrap()
        );
        title_text = bored.get_name().to_owned() + "\n" + &bored_name;
        // status_text = "Connected, bored loded";
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
            // eprintln!("{:?}", bored_view_buffer);
            frame.buffer_mut().merge(&bored_view_buffer);
        }
        app.bored_view_port = Some(bored_view_port);
    }
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::QuadrantOutside)
        .style(app.theme.header_style())
        .bold();
    // let title_rect = Rect::new(0, 0, area.width, 4);
    let title = Paragraph::new(Text::raw(title_text)).block(title_block);
    frame.render_widget(title, ui_chunks[0]);

    // modify based on current_view
    match &app.current_view {
        View::ErrorView(e) => {
            let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4)); //centered_rect(60, 60, area);
            let navigation_text = "Press (enter) to contiune or (q) to quit.";
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
                // .title("Error")
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
        View::Waiting(message) => {
            let pop_up_rect = area.inner(Margin::new(area.width / 4, area.height / 4));
            Clear.render(pop_up_rect, frame.buffer_mut());
            let pop_up_block = Block::default()
                .title("Working...")
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .style(app.theme.text_style());
            // frame.render_widget(pop_up_block, pop_up_rect);
            let pop_up_text = Paragraph::new(Text::styled(format!("{message}"), Style::default()))
                .block(pop_up_block);
            // .wrap(Wrap { trim: false });
            frame.render_widget(pop_up_text, pop_up_rect);
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
                let bored = app
                    .get_current_bored()
                    .expect("There should not be a draft without a bored");
                match draft_mode {
                    DraftMode::Content => {
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
        View::BoredView => {}
        View::NoticeView { hyperlinks_index } => {
            if let Some(notice) = app.get_selected_notice() {
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
        _ => (),
    }
    // setup status area
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::QuadrantOutside)
        .style(app.theme.header_style())
        .bold();
    // let status_rect = Rect::new(0, area.height - 5, area.width, 5);
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
