use app::SurfBoredError;
use bored::{
    Bored, BoredAddress, BoredError, Coordinate, Direction, bored_client,
    notice::{self, MAX_URL_LENGTH},
};
use core::panic;
use rand::Rng;
use ratatui::{
    Terminal, Viewport,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        cursor::position,
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
            KeyModifiers, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
            PushKeyboardEnhancementFlags,
        },
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Margin, Rect, Size},
    style::Modifier,
};
use std::{char::MAX, cmp::max, cmp::min, error::Error, io};

mod app;
mod display_bored;
mod ui;
use crate::app::{App, CreateMode, DraftMode, GoToMode, HyperlinkMode, View};
use crate::ui::ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // create app and try to init client if fail error will stop program
    let mut app = App::new();
    println!("Trying to connect to antnet...");
    app.init_client().await?;
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal, &mut app).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        PopKeyboardEnhancementFlags
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_app<B: Backend>(
    termimal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    //io::Result<()> {
    if let Err(e) = app.load_directory() {
        app.display_error(e);
    }
    if let Some(home_address) = app.directory.get_home() {
        let home_address = match BoredAddress::from_string(home_address.to_string()) {
            Ok(home_address) => match app.goto_bored(home_address).await {
                Err(e) => app.display_error(e),
                _ => (),
            },
            Err(e) => app.display_error(app::SurfBoredError::BoredError(e)),
        };
    }

    loop {
        termimal.draw(|f| ui(f, app))?;
        if let Event::Key(key) = event::read()? {
            // app.status = format!("{:?}", key);
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
                        KeyCode::Char('q') => break,
                        _ => {}
                    },
                    View::BoredView => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('w') => try_select_notice(app, bored::Direction::Up),
                        KeyCode::Char('a') => try_select_notice(app, bored::Direction::Left),
                        KeyCode::Char('s') => try_select_notice(app, bored::Direction::Down),
                        KeyCode::Char('d') => try_select_notice(app, bored::Direction::Right),
                        KeyCode::Char('c') => app.change_view(View::CreateView(CreateMode::Name)),
                        KeyCode::Char('n') => {
                            if let Some(bored) = app.get_current_bored() {
                                let terminal_size = termimal.size()?;
                                let bored_dimensions = bored.get_dimensions();
                                let draft_dimensions =
                                    generate_notice_size(terminal_size, bored_dimensions);
                                match app.create_draft(draft_dimensions) {
                                    Err(e) => {
                                        app.current_view =
                                            View::ErrorView(app::SurfBoredError::BoredError(e))
                                    }
                                    _ => (),
                                }
                                // postion draft centered in current view in UI
                                let view_rect = match &app.bored_view_port {
                                    Some(bored_view_port) => bored_view_port.get_view(),
                                    None => Rect::new(0, 0, bored_dimensions.x, bored_dimensions.y),
                                };
                                let x = ((min(view_rect.width, bored_dimensions.x)
                                    - draft_dimensions.x)
                                    / 2)
                                    + view_rect.x;
                                let y = ((min(view_rect.height, bored_dimensions.y)
                                    - draft_dimensions.y)
                                    / 2)
                                    + view_rect.y;
                                match app.position_draft(Coordinate { x, y }) {
                                    Err(e) => {
                                        app.current_view =
                                            View::ErrorView(app::SurfBoredError::BoredError(e))
                                    }
                                    _ => (),
                                }
                            } else {
                                // if bored doesn't exist go back to previous view
                                app.revert_view();
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
                                let new_bored = app
                                    .create_bored_on_network(
                                        &app.name_input.clone(),
                                        &app.key_input.clone(),
                                        Coordinate { x: 120, y: 40 },
                                    )
                                    .await;
                                match new_bored {
                                    Err(e) => app.display_error(e),
                                    _ => (),
                                }
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
                                        if let Err(bored_error) = app.add_draft_to_bored().await {
                                            app.current_view = View::ErrorView(
                                                app::SurfBoredError::BoredError(bored_error),
                                            );
                                        } else {
                                            app.content_input = String::new();
                                            app.change_view(View::BoredView);
                                        }
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
fn try_select_notice(app: &mut App, direction: Direction) {
    app.select_notice(direction);
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
            //otherwise in middle of screen
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
