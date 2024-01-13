use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
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
    on_input: bool,
    input: String,
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
            on_input: false,
            input: String::new(),
        }
    }

    fn input_char(&mut self, c: char) {
        self.input.push(c);
    }

    fn input_backspace(&mut self) {
        self.input.pop();
    }

    fn input_enter(&mut self) -> Result<()> {
        self.on_input = false;
        core::add_branch(self.project_path.as_str(), self.input.clone())?;
        self.input.clear();
        self.reload_items()?;
        Ok(())
    }

    fn input_esc(&mut self) {
        self.on_input = false;
        self.input.clear();
    }

    fn reload_items(&mut self) -> Result<()> {
        self.items = StatefulList::with_items(
            core::get_branches(self.project_path.as_str())?
                .iter()
                .map(|b| b.name.clone())
                .collect::<Vec<String>>(),
        );
        Ok(())
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
                    if app.on_input {
                        match key.code {
                            KeyCode::Esc => app.input_esc(),
                            KeyCode::Enter => app.input_enter()?,
                            KeyCode::Char(c) => app.input_char(c),
                            KeyCode::Backspace => app.input_backspace(),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('a') => {
                                app.on_input = true;
                            }
                            KeyCode::Down | KeyCode::Char('j') => app.items.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.items.previous(),
                            KeyCode::Char('r') => {
                                let selected = app
                                    .items
                                    .state
                                    .selected()
                                    .ok_or(anyhow!("no branch selected"))?;
                                let branch = app.items.items[selected].clone();
                                core::remove_branch(app.project_path.as_str(), branch)?;
                                app.reload_items()?;
                            }
                            KeyCode::Enter => {
                                let selected = app
                                    .items
                                    .state
                                    .selected()
                                    .ok_or(anyhow!("no branch selected"))?;
                                let branch = app.items.items[selected].as_str();
                                core::checkout(app.project_path.as_str(), branch)?;
                                return Ok(());
                            }
                            _ => {}
                        }
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

    if app.on_input {
        let input = Paragraph::new(app.input.as_str())
            .block(Block::default().title("Add branch").borders(Borders::ALL));
        f.render_widget(input, screen);
    } else {
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
}
