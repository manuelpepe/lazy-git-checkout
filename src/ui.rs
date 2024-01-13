// https://github.com/ratatui-org/ratatui/blob/main/examples/list.rs

use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState},
    Frame, Terminal,
};

use crate::{core, core::Project};

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}
impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

struct UI {
    project_path: String,
    items: StatefulList<String>,
}

impl UI {
    fn new(project: &Project) -> UI {
        let items = project
            .branches
            .iter()
            .map(|b| b.name.clone())
            .collect::<Vec<String>>();

        UI {
            project_path: project.path.clone(),
            items: StatefulList::with_items(items),
        }
    }
}

pub fn start_ui(project: Project) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = UI::new(&project);
    let res = run_ui(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_ui<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    mut app: UI,
    tick_rate: Duration,
) -> Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down | KeyCode::Char('j') => app.items.next(),
                        KeyCode::Up | KeyCode::Char('k') => app.items.previous(),
                        KeyCode::Char('r') => {
                            let selected = app.items.state.selected().unwrap_or(0);
                            let branch = app.items.items[selected].clone();
                            core::remove_branch(app.project_path.as_str(), branch)?;
                            app.items = StatefulList::with_items(
                                core::get_branches(app.project_path.as_str())?
                                    .iter()
                                    .map(|b| b.name.clone())
                                    .collect::<Vec<String>>(),
                            );
                        }
                        KeyCode::Enter => {
                            let selected = app.items.state.selected().unwrap_or(0);
                            let branch = app.items.items[selected].as_str();
                            core::checkout(app.project_path.as_str(), branch)?;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn draw(f: &mut Frame, app: &mut UI) {
    let screen = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size())[0];

    let items: Vec<ListItem> = app
        .items
        .items
        .iter()
        .map(|opt| ListItem::new(opt.clone()))
        .collect();

    let items = List::new(items)
        .block(Block::default().title(app.project_path.clone()))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(items, screen, &mut app.items.state);
}
