use bored::notice::MAX_URL_LENGTH;
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::Margin,
};
use std::{error::Error, io};

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
    execute!(stdout, EnterAlternateScreen)?;
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
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_app<B: Backend>(termimal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    if let Err(e) = app.load_directory() {
        app.display_error(e);
    }
    loop {
        termimal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEvenKind::Press
                continue;
            }
            if key.code == KeyCode::Char('q') {
                // can't have this active while editing text
                break;
            }
            match &app.current_view {
                View::ErrorView(_) => match key.code {
                    KeyCode::Enter => app.current_view = app.previous_view.clone(),
                    _ => {}
                },
                View::BoredView(_) => match key.code {
                    KeyCode::Char('c') => app.current_view = View::CreateView(CreateMode::Name),
                    _ => {}
                },
                View::CreateView(create_view) => match key.code {
                    KeyCode::Tab => app.current_view = View::CreateView(create_view.toggle()),
                    KeyCode::Esc => app.current_view = app.previous_view.clone(),
                    KeyCode::Backspace => match create_view {
                        CreateMode::Name => {
                            app.input_1.pop();
                        }
                        CreateMode::PrivateKey => {
                            app.input_2.pop();
                        }
                    },
                    KeyCode::Char(value) => match create_view {
                        CreateMode::Name => app.input_1.push(value),
                        CreateMode::PrivateKey => app.input_2.push(value),
                    },
                    KeyCode::Enter => match create_view {
                        CreateMode::Name => {
                            app.current_view = View::CreateView(CreateMode::PrivateKey)
                        }
                        CreateMode::PrivateKey => {
                            let new_bored = app
                                .create_bored_on_network(&app.input_1.clone(), &app.input_2.clone())
                                .await;
                            match new_bored {
                                Err(e) => app.display_error(e),
                                _ => (),
                            }
                        }
                    },
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}
