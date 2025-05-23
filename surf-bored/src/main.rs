use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
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
                break;
            }
            match &app.current_view {
                View::ErrorView(_) => match key.code {
                    KeyCode::Enter => app.current_view = app.previous_view.clone(),
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}
