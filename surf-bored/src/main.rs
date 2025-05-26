use bored::{Bored, BoredAddress, BoredError, Coordinate, bored_client, notice::MAX_URL_LENGTH};
use rand::Rng;
use ratatui::{
    Terminal, Viewport,
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
    layout::{Margin, Size},
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

async fn run_app<B: Backend>(termimal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
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
                                    if value == 'a' {
                                        app.current_view = View::DraftView(DraftMode::Position);
                                    }
                                }
                                app.content_input.push(value);
                                try_edit(app);
                            }

                            _ => {}
                        },
                        DraftMode::Hyperlink(hyperlink_mode) => match key.code {
                            KeyCode::Esc => app.current_view = View::DraftView(DraftMode::Content),
                            _ => {}
                        },
                        // DraftMode::Position => {

                        // }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
    Ok(())
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
    let x = max(9, max_x / 7);
    let y = max(3, max_y / 7);
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
