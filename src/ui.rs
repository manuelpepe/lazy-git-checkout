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
    widgets::{Block, List, ListItem},
    Frame, Terminal,
};

use crate::{
    core,
    core::Project,
    widgets::{AddBranchWidget, ExitContextResult, StatefulList},
};

enum Mode {
    Normal,
    Input,
}

struct UI {
    project_path: String,
    saved_branches: StatefulList<String>,

    mode: Mode,

    add_branches_widget: AddBranchWidget,
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
            add_branches_widget: AddBranchWidget::new(all_branches.clone()),
        }
    }

    fn on_char(&mut self, c: char) {
        match self.mode {
            Mode::Input => self.add_branches_widget.input_char(c),
            Mode::Normal => {}
        }
    }

    fn on_backspace(&mut self) {
        match self.mode {
            Mode::Input => self.add_branches_widget.remove_char(),
            Mode::Normal => {}
        }
    }

    fn on_enter(&mut self) -> Result<()> {
        match self.mode {
            Mode::Input => {
                let new_branch = self.add_branches_widget.get_branch_name();
                if new_branch.is_empty() {
                    return Ok(());
                }
                core::add_branch(self.project_path.as_str(), new_branch)?;
                self.reload_saved_branches()?;
                self.mode = Mode::Normal;
                Ok(())
            }
            Mode::Normal => {
                let selected = self
                    .saved_branches
                    .selected()
                    .ok_or(anyhow!("no branch selected"))?;
                let branch = self.saved_branches.items()[selected].as_str();
                core::checkout(self.project_path.as_str(), branch)?;
                Ok(())
            }
        }
    }

    fn on_esc(&mut self) {
        match self.add_branches_widget.exit_context() {
            ExitContextResult::Exit => {
                self.mode = Mode::Normal;
            }
            ExitContextResult::Continue => {}
        }
    }

    fn on_up(&mut self) {
        match self.mode {
            Mode::Input => self.add_branches_widget.previous(),
            Mode::Normal => self.saved_branches.previous(),
        }
    }

    fn on_down(&mut self) {
        match self.mode {
            Mode::Input => self.add_branches_widget.next(),
            Mode::Normal => self.saved_branches.next(),
        }
    }

    fn on_remove_branch(&mut self) -> Result<()> {
        let selected = self
            .saved_branches
            .selected()
            .ok_or(anyhow!("no branch selected"))?;
        let branch = self.saved_branches.items()[selected].clone();
        core::remove_branch(self.project_path.as_str(), branch)?;
        self.reload_saved_branches()?;
        Ok(())
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
                            KeyCode::Esc => app.on_esc(),
                            KeyCode::Enter => app.on_enter()?,
                            KeyCode::Char(c) => app.on_char(c),
                            KeyCode::Backspace => app.on_backspace(),
                            KeyCode::Down => app.on_down(),
                            KeyCode::Up => app.on_up(),
                            _ => {}
                        },
                        Mode::Normal => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('a') => app.mode = Mode::Input,
                            KeyCode::Down | KeyCode::Char('j') => app.on_down(),
                            KeyCode::Up | KeyCode::Char('k') => app.on_up(),
                            KeyCode::Char('r') => app.on_remove_branch()?,
                            KeyCode::Enter => {}
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
            app.add_branches_widget.draw(f, screen);
        }
        Mode::Normal => {
            let items: Vec<ListItem> = app
                .saved_branches
                .items()
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
