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

use app::{NoticeSelection, SurfBoredError};
use bored::{
    Bored, BoredAddress, BoredError, Coordinate, Direction, bored_client,
    bored_client::ConnectionType,
    notice::{self, MAX_URL_LENGTH},
};
use core::arch;
use directory::Directory;
use rand::{Rng, seq::IndexedRandom};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
            KeyModifiers, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
            PushKeyboardEnhancementFlags,
        },
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Margin, Rect, Size},
};
use std::{
    cmp::{max, min},
    env::Args,
    error::Error,
    fs, io,
    time::Duration,
};
use tokio::time::sleep;

mod app;
mod directory;
mod display_bored;
mod theme;
mod ui;
use crate::app::{App, CreateMode, DraftMode, HyperlinkMode, View};
use crate::ui::{safe_subtract_u16, ui, wait_pop_up};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // create app and try to init client if fail error will stop program
    let args: Vec<String> = std::env::args().collect();
    let mut app = App::new();
    println!("Trying to connect to antnet...");
    let mut connection_type = ConnectionType::Antnet;
    if args.len() > 1 {
        if &args[1] == "local" {
            connection_type = ConnectionType::Local;
        }
    }
    app.init_client(connection_type).await?;
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen,)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal, &mut app).await?;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    // loop {
    // terminal.draw(|f| ui(f, app))?;
    let previous_buffer = terminal.draw(|f| ui(f, app))?.buffer.clone();
    if let Err(e) = app.load_directory() {
        app.directory = Directory::default();
        app.save_directory()?;
        // app.display_error(e);
    }
    if let Some(home_address) = app.directory.get_home() {
        match BoredAddress::from_string(home_address.to_string()) {
            Ok(home_address) => {
                let theme = app.theme.clone();
                let going_to_bored = app.goto_bored(home_address);
                match wait_pop_up(
                    terminal,
                    previous_buffer,
                    going_to_bored,
                    "Loading bored from antnet...",
                    theme,
                )
                .await
                {
                    Err(e) => app.display_error(e),
                    _ => (),
                }
            }
            Err(e) => app.display_error(app::SurfBoredError::BoredError(e)),
        };
    }

    loop {
        let previous_buffer = terminal.draw(|f| ui(f, app))?.buffer.clone();
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEvenKind::Press
                continue;
            }
            if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                break;
            } else {
                match &app.current_view {
                    View::ErrorView(_) => match key.code {
                        KeyCode::Enter => app.revert_view(),
                        KeyCode::Esc => app.revert_view(),
                        KeyCode::Char('q') => break,
                        _ => {}
                    },
                    View::BoredView => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Tab => try_select_notice(app, NoticeSelection::Next),
                        KeyCode::BackTab => try_select_notice(app, NoticeSelection::Previous),
                        KeyCode::Up => {
                            try_select_notice(app, NoticeSelection::Direction(bored::Direction::Up))
                        }
                        KeyCode::Left => try_select_notice(
                            app,
                            NoticeSelection::Direction(bored::Direction::Left),
                        ),
                        KeyCode::Down => try_select_notice(
                            app,
                            NoticeSelection::Direction(bored::Direction::Down),
                        ),
                        KeyCode::Right => try_select_notice(
                            app,
                            NoticeSelection::Direction(bored::Direction::Right),
                        ),
                        KeyCode::Enter => {
                            app.selected_notice.inspect(|_| {
                                app.change_view(View::NoticeView {
                                    hyperlinks_index: None,
                                })
                            });
                        }
                        KeyCode::Char('c') => app.change_view(View::CreateView(CreateMode::Name)),
                        KeyCode::Char('n') => {
                            if let Some(bored) = app.get_current_bored() {
                                let terminal_size = terminal.size()?;
                                let bored_dimensions = bored.get_dimensions();
                                let draft_dimensions =
                                    generate_notice_size(terminal_size, bored_dimensions);
                                match app.create_draft(draft_dimensions) {
                                    Err(e) => app.change_view(View::ErrorView(
                                        app::SurfBoredError::BoredError(e),
                                    )),
                                    _ => (),
                                }
                                // postion draft centered in current view in UI
                                let view_rect = match &app.bored_view_port {
                                    Some(bored_view_port) => bored_view_port.get_view(),
                                    None => Rect::new(0, 0, bored_dimensions.x, bored_dimensions.y),
                                };
                                let x = (safe_subtract_u16(
                                    min(view_rect.width, bored_dimensions.x),
                                    draft_dimensions.x,
                                ) / 2)
                                    + view_rect.x;
                                let y = (safe_subtract_u16(
                                    min(view_rect.height, bored_dimensions.y),
                                    draft_dimensions.y,
                                ) / 2)
                                    + view_rect.y;
                                match app.position_draft(Coordinate { x, y }) {
                                    Err(e) => app.change_view(View::ErrorView(
                                        app::SurfBoredError::BoredError(e),
                                    )),
                                    _ => (),
                                }
                            } else {
                                // if bored doesn't exist go back to previous view
                                app.revert_view();
                            }
                        }
                        KeyCode::Char('g') => app.change_view(View::GoToView),
                        KeyCode::Char('d') => app.change_view(View::DirectoryView(0)),
                        _ => {}
                    },
                    View::NoticeView { .. } => match key.code {
                        KeyCode::Esc => app.revert_view(), //app.current_view = View::BoredView,
                        KeyCode::Char('q') => break,
                        KeyCode::Tab => app.next_hyperlink(),
                        KeyCode::BackTab => app.previous_hyperlink(),
                        KeyCode::Enter => {
                            if let Some(hyperlink) = app.get_selected_hyperlink() {
                                match BoredAddress::from_string(hyperlink.get_link()) {
                                    Ok(bored_address) => {
                                        let theme = app.theme.clone();
                                        let going_to_bored = app.goto_bored(bored_address);
                                        match wait_pop_up(
                                            terminal,
                                            previous_buffer,
                                            going_to_bored,
                                            "Loading bored from antnet...",
                                            theme,
                                        )
                                        .await
                                        {
                                            Err(e) => app.display_error(e),
                                            _ => (),
                                        }
                                        app.revert_view();
                                    }
                                    Err(e) => {
                                        app.display_error(SurfBoredError::BoredError(e));
                                    }
                                }
                            }
                        }
                        KeyCode::Char('o') => {
                            fs::write("notice", format!("{:?}", app.get_selected_notice()))?;
                        }
                        _ => {}
                    },
                    View::GoToView => match key.code {
                        KeyCode::Esc => app.revert_view(),
                        KeyCode::Backspace => {
                            app.goto_input.pop();
                        }
                        KeyCode::Char(value) => app.goto_input.push(value),
                        KeyCode::Enter => {
                            match BoredAddress::from_string(app.goto_input.to_string()) {
                                Ok(address) => {
                                    let theme = app.theme.clone();
                                    let going_to_bored = app.goto_bored(address);
                                    match wait_pop_up(
                                        terminal,
                                        previous_buffer,
                                        going_to_bored,
                                        "Loading bored from antnet...",
                                        theme,
                                    )
                                    .await
                                    {
                                        Err(e) => app.display_error(e),
                                        _ => app.goto_input = String::new(),
                                    }
                                }
                                Err(e) => app.display_error(app::SurfBoredError::BoredError(e)),
                            };
                        }
                        _ => {}
                    },
                    &View::DirectoryView(directory_index) => match key.code {
                        KeyCode::Esc => app.revert_view(),
                        KeyCode::Up => {
                            let new_directroy_index =
                                app.previous_directory_item(directory_index)?;
                            app.change_view(View::DirectoryView(new_directroy_index));
                        }
                        KeyCode::Down => {
                            let new_directroy_index = app.next_directory_item(directory_index)?;
                            app.change_view(View::DirectoryView(new_directroy_index));
                        }
                        KeyCode::Char('h') => {
                            if key.modifiers == KeyModifiers::CONTROL {
                                app.directory.set_home(directory_index);
                            }
                        }
                        KeyCode::Enter => {
                            let bored_address = app.directory.get_bored_address(directory_index)?;
                            match app.interupted_view {
                                View::BoredView => {
                                    match BoredAddress::from_string(bored_address.bored_address) {
                                        Ok(address) => {
                                            let theme = app.theme.clone();
                                            let going_to_bored = app.goto_bored(address);
                                            match wait_pop_up(
                                                terminal,
                                                previous_buffer,
                                                going_to_bored,
                                                "Loading bored from antnet...",
                                                theme,
                                            )
                                            .await
                                            {
                                                Err(e) => app.display_error(e),
                                                _ => app.goto_input = String::new(),
                                            }
                                        }
                                        Err(e) => {
                                            app.display_error(app::SurfBoredError::BoredError(e))
                                        }
                                    };
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    },
                    View::CreateView(create_view) => match key.code {
                        // not using change_view() becase don;t wnat to change previous view for sub enum change
                        KeyCode::Tab => app.current_view = View::CreateView(create_view.toggle()),
                        KeyCode::Esc => app.revert_view(),
                        KeyCode::Backspace => match create_view {
                            CreateMode::Name => {
                                app.name_input.pop();
                            }
                            CreateMode::PrivateKey => {
                                app.key_input.pop();
                            }
                        },
                        KeyCode::Char(value) => match create_view {
                            CreateMode::Name => app.name_input.push(value),
                            CreateMode::PrivateKey => app.key_input.push(value),
                        },
                        KeyCode::Enter => match create_view {
                            CreateMode::Name => {
                                app.current_view = View::CreateView(CreateMode::PrivateKey)
                            }
                            CreateMode::PrivateKey => {
                                // app.change_view(View::Waiting(
                                //     "Creating bored on antnet".to_string(),
                                // ));
                                let (name_input, key_input) =
                                    (app.name_input.clone(), app.key_input.clone());
                                let theme = app.theme.clone();
                                let creating_bored = app.create_bored_on_network(
                                    &name_input,
                                    &key_input,
                                    Coordinate { x: 120, y: 40 },
                                );
                                match wait_pop_up(
                                    terminal,
                                    previous_buffer,
                                    creating_bored,
                                    "Creating bored on antnet...",
                                    theme,
                                )
                                .await
                                {
                                    Err(e) => app.display_error(e),
                                    _ => app.name_input = String::new(),
                                }
                                app.key_input = String::new();
                            }
                        },
                        _ => {}
                    },
                    // due to wrapping it may still allow some non visible text to be typed
                    // it would still be within the specfication just not visible in this app
                    View::DraftView(draft_mode) => match draft_mode {
                        DraftMode::Content => match key.code {
                            KeyCode::Esc => app.revert_view(),
                            KeyCode::Backspace => {
                                app.content_input.pop();
                                app.edit_draft(&app.content_input.clone())
                                    .expect("Shoud never be more text as deleting")
                            }
                            KeyCode::Enter => {
                                app.content_input.push('\n');
                                // this doesn't seem to remove the first newline that is not visible
                                try_edit(app);
                            }
                            KeyCode::Char(value) => {
                                if key.modifiers == KeyModifiers::CONTROL {
                                    if value == 'h' {
                                        app.current_view = View::DraftView(DraftMode::Hyperlink(
                                            HyperlinkMode::Text,
                                        ));
                                    }
                                    if value == 'p' {
                                        app.current_view = View::DraftView(DraftMode::Position);
                                    }
                                    if value == 'u' {
                                        app.content_input = String::new();
                                    }
                                }
                                if app.current_view == View::DraftView(DraftMode::Content) {
                                    app.content_input.push(value);
                                    try_edit(app);
                                }
                            }

                            _ => {}
                        },
                        DraftMode::Hyperlink(hyperlink_mode) => match key.code {
                            KeyCode::Esc => app.current_view = View::DraftView(DraftMode::Content),
                            _ => {}
                        },
                        DraftMode::Position => {
                            if key.code == KeyCode::Esc {
                                app.current_view = View::DraftView(DraftMode::Content);
                            }
                            if let Some(mut draft) = app.get_draft() {
                                let bored = app
                                    .get_current_bored()
                                    .expect("Bored should exist if there is a draft");
                                let position = draft.get_top_left();
                                match key.code {
                                    KeyCode::Up => try_move(
                                        app,
                                        position.subtact(&Coordinate { x: 0, y: 1 }),
                                        (0, -1),
                                    ),
                                    KeyCode::Down => try_move(
                                        app,
                                        position.add(&Coordinate { x: 0, y: 1 }),
                                        (0, 1),
                                    ),
                                    KeyCode::Left => try_move(
                                        app,
                                        position.subtact(&Coordinate { x: 1, y: 0 }),
                                        (-1, 0),
                                    ),
                                    KeyCode::Right => try_move(
                                        app,
                                        position.add(&Coordinate { x: 1, y: 0 }),
                                        (1, 0),
                                    ),
                                    KeyCode::Enter => {
                                        let theme = app.theme.clone();
                                        let going_onto_bored = app.add_draft_to_bored();
                                        match wait_pop_up(
                                            terminal,
                                            previous_buffer,
                                            going_onto_bored,
                                            "Updating bored on antnet...",
                                            theme,
                                        )
                                        .await
                                        {
                                            Err(e) => app.display_error(e),
                                            _ => app.change_view(View::BoredView),
                                        }
                                        app.content_input = String::new();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn try_select_notice(app: &mut App, notice_selection: NoticeSelection) {
    match notice_selection {
        NoticeSelection::Direction(direction) => app.select_notice(direction),
        NoticeSelection::Next => app.increment_selected_notice(),
        NoticeSelection::Previous => app.decrement_selected_notice(),
    }
    if let Some(notice) = app.get_selected_notice() {
        let bored_view_port = app
            .bored_view_port
            .as_mut()
            .expect("Bored view port should exist by now");
        if !bored_view_port.in_view(
            notice.get_top_left(),
            notice.get_top_left().add(&notice.get_dimensions()),
        ) {
            //if at bottom right show as much of bored as possible
            // otherwise in middle of screen
            // otherwise at notices top left
            let new_view_position = bored_view_port.get_view_for_notice(&notice);
            bored_view_port.move_view(new_view_position);
        }
    }
}

fn try_move(app: &mut App, new_position: Coordinate, scroll_offset: (i32, i32)) {
    // Do nothing is error so user can't move notice outside of bored
    match app.position_draft(new_position) {
        Ok(in_view) => {
            // if bottom right of notice is off screen scoll view towards it
            if !in_view {
                if let Some(bored_view_port) = app.bored_view_port.as_mut() {
                    let mut new_view_position = bored_view_port.get_view_top_left();
                    new_view_position = new_view_position.add_i32_tuple(scroll_offset);
                    bored_view_port.move_view(new_view_position);
                }
            }
        }
        _ => (),
    }
}

fn try_edit(app: &mut App) {
    if let Err(e) = app.edit_draft(&app.content_input.clone()) {
        match e {
            // if to much text don't let user type any more
            BoredError::TooMuchText => {
                app.content_input.pop();
            }
            _ => (),
        };
    }
}

fn generate_notice_size(terminal_size: Size, bored_size: Coordinate) -> Coordinate {
    let max_x = min(terminal_size.width, bored_size.x);
    let max_y = min(terminal_size.height, bored_size.y);
    let x = max(9, max_x / 4);
    let y = max(3, max_y / 4);
    Coordinate { x, y }

    // // portrait or landscape
    // if rand::rng().random_range(0..1) == 0 {
    //     // neet to take into account bored size as well
    //     width = rand::rng().random_range(
    //         max(9, terminal_size.width / 6)
    //             ..min(60, min(terminal_size.width, bored_dimensions.x) / 2),
    //     );
    //     height = rand::rng().random_range(
    //         max(6, terminal_size.height / 3)
    //             ..min(
    //                 18,
    //                 min(terminal_size.height, bored_dimensions.y) / 2,
    //             ),
    //     );
    // } else {
    //     width = rand::rng().random_range(
    //         max(12, terminal_size.width / 4)
    //             ..min(90, min(terminal_size.width, bored_dimensions.x) / 2),
    //     );
    //     height = rand::rng().random_range(
    //         max(4, terminal_size.height / 5)
    //             ..min(
    //                 12,
    //                 min(terminal_size.height, bored_dimensions.y) / 2,
    //             ),
    //     );
    // }
}
