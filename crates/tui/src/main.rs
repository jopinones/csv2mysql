use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;
use walkdir::WalkDir;

mod app;
use app::App;

const HELP_TEXT: &str = r#"
csv2mysql - TUI para carga de CSV a MySQL

ATAJOS:
  ↑/↓     - Navegar árbol
  →/Enter - Expandir/seleccionar
  ←       - Colapsar
  E       - Ejecutar carga
  D       - Dry-run
  V       - Validar
  q       - Salir

F1 - Ayuda
"#;

pub fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(PathBuf::from("."));
    
    // Run app loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Up => app.previous(),
                    KeyCode::Down => app.next(),
                    KeyCode::Enter | KeyCode::Right => app.select(),
                    KeyCode::Left => app.back(),
                    KeyCode::Char('e') | KeyCode::Char('E') => app.execute(),
                    KeyCode::Char('d') | KeyCode::Char('D') => app.dry_run(),
                    KeyCode::Char('v') | KeyCode::Char('V') => app.validate(),
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new("csv2mysql v0.1.0 - Carga de CSV a MySQL")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(chunks[1]);

    // Left panel - File tree
    render_file_tree(f, app, main_chunks[0]);

    // Right panel - Details
    render_details(f, app, main_chunks[1]);

    // Footer - Hotkeys
    let footer = Paragraph::new("E:ejecutar | D:dry-run | V:validar | ↑↓:navegar | q:salir")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

fn render_file_tree(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .files
        .iter()
        .map(|file| {
            let name = file.file_name().unwrap_or_default().to_string_lossy();
            let prefix = if file.is_dir() { "📁 " } else { "📄 " };
            ListItem::new(Line::from(format!("{}{}", prefix, name)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Archivos"))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(226, 125, 96))
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn render_details(f: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(file) = app.files.get(app.selected) {
        if file.is_file() {
            format!(
                "Archivo seleccionado:\n\n\
                Ruta: {}\n\
                Tamaño: {} bytes\n\n\
                Presiona E para ejecutar la carga\n\
                Presiona D para dry-run\n\
                Presiona V para solo validar",
                file.display(),
                file.metadata().map(|m| m.len()).unwrap_or(0)
            )
        } else {
            "Selecciona un archivo CSV para ver detalles".to_string()
        }
    } else {
        "No hay archivos disponibles".to_string()
    };

    let details = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Detalles"))
        .wrap(Wrap { trim: true });

    f.render_widget(details, area);
}

fn main() -> Result<()> {
    run_tui()
}
