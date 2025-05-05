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
mod ui;
use crate::app::{App, CreateMode, DraftMode, GoToMode, HyperlinkMode, View};
use crate::ui::ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // setu terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::init().await?;
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
    loop {
        app.create_bored_on_network("Testy bored", "")
            .await
            .unwrap();
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
                View::ErrorView(bored_error) => match key.code {
                    KeyCode::Enter => app.current_view = app.previous_view.clone(),
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}
