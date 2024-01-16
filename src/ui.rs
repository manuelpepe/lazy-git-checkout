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
    layout::{Constraint, Direction, Layout},
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

    fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.state.select(Some(0));
    }

    fn clear(&mut self) {
        self.items.clear();
        self.state.select(None);
    }

    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
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
        if self.items.is_empty() {
            return;
        }
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

enum Mode {
    Normal,
    Input,
}

struct UI {
    project_path: String,
    saved_branches: StatefulList<String>,

    mode: Mode,

    all_branches: Vec<String>,
    add_branch_input: String,
    add_branch_autocomplete: StatefulList<String>,
}

impl UI {
    fn new(project: &Project, all_branches: Vec<String>) -> UI {
        let saved_branches = project
            .branches
            .iter()
            .map(|b| b.name.clone())
            .collect::<Vec<String>>();

        UI {
            project_path: project.path.clone(),
            saved_branches: StatefulList::with_items(saved_branches),
            mode: Mode::Normal,
            add_branch_input: String::new(),
            all_branches: all_branches.clone(),
            add_branch_autocomplete: StatefulList::with_items(all_branches),
        }
    }

    fn update_autocomplete(&mut self) {
        let items = self
            .all_branches
            .iter()
            .filter(|b| b.starts_with(self.add_branch_input.as_str()))
            .cloned()
            .collect::<Vec<String>>();
        self.add_branch_autocomplete.set_items(items);
        self.add_branch_autocomplete.state.select(None)
    }

    fn input_char(&mut self, c: char) {
        self.add_branch_input.push(c);
        self.update_autocomplete();
    }

    fn input_backspace(&mut self) {
        self.add_branch_input.pop();
        self.update_autocomplete();
    }

    fn input_enter(&mut self) -> Result<()> {
        self.mode = Mode::Normal;
        let new_branch = match self.add_branch_autocomplete.state.selected() {
            Some(i) => self.add_branch_autocomplete.items[i].clone(),
            None => self.add_branch_input.clone(),
        };
        core::add_branch(self.project_path.as_str(), new_branch)?;
        self.add_branch_input.clear();
        self.add_branch_autocomplete.clear();
        self.reload_saved_branches()?;
        Ok(())
    }

    fn input_esc(&mut self) {
        match self.add_branch_autocomplete.state.selected() {
            Some(_) => {
                self.add_branch_autocomplete.state.select(None);
            }
            None => {
                self.mode = Mode::Normal;
                self.add_branch_input.clear();
            }
        }
    }

    fn reload_saved_branches(&mut self) -> Result<()> {
        self.saved_branches = StatefulList::with_items(
            core::get_branches(self.project_path.as_str())?
                .iter()
                .map(|b| b.name.clone())
                .collect::<Vec<String>>(),
        );
        Ok(())
    }
}

pub fn start_ui(project: Project, branches: Vec<String>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = UI::new(&project, branches);
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
                    match app.mode {
                        Mode::Input => match key.code {
                            KeyCode::Esc => app.input_esc(),
                            KeyCode::Enter => app.input_enter()?,
                            KeyCode::Char(c) => app.input_char(c),
                            KeyCode::Backspace => app.input_backspace(),
                            KeyCode::Down => app.add_branch_autocomplete.next(),
                            KeyCode::Up => app.add_branch_autocomplete.previous(),
                            _ => {}
                        },
                        Mode::Normal => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('a') => app.mode = Mode::Input,
                            KeyCode::Down | KeyCode::Char('j') => app.saved_branches.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.saved_branches.previous(),
                            KeyCode::Char('r') => {
                                let selected = app
                                    .saved_branches
                                    .state
                                    .selected()
                                    .ok_or(anyhow!("no branch selected"))?;
                                let branch = app.saved_branches.items[selected].clone();
                                core::remove_branch(app.project_path.as_str(), branch)?;
                                app.reload_saved_branches()?;
                            }
                            KeyCode::Enter => {
                                let selected = app
                                    .saved_branches
                                    .state
                                    .selected()
                                    .ok_or(anyhow!("no branch selected"))?;
                                let branch = app.saved_branches.items[selected].as_str();
                                core::checkout(app.project_path.as_str(), branch)?;
                                return Ok(());
                            }
                            _ => {}
                        },
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

    match app.mode {
        Mode::Input => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
                .split(screen);

            let input = Paragraph::new(app.add_branch_input.as_str())
                .block(Block::default().title("Add branch").borders(Borders::ALL));

            let items: Vec<ListItem> = app
                .add_branch_autocomplete
                .items
                .iter()
                .map(|opt| ListItem::new(opt.clone()))
                .collect();

            let autocomplete_list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            f.render_widget(input, chunks[0]);
            f.render_stateful_widget(
                autocomplete_list,
                chunks[1],
                &mut app.add_branch_autocomplete.state,
            );
        }
        Mode::Normal => {
            let items: Vec<ListItem> = app
                .saved_branches
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

            f.render_stateful_widget(items, screen, &mut app.saved_branches.state);
        }
    }
}
