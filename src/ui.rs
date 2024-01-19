use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    Frame, Terminal,
};

use crate::{
    core::Project,
    widgets::{AddBranchWidget, ChangeBranchesWidget, ChangeBranchesWidgetMode, ExitContextResult},
};

macro_rules! continue_after {
    ($expr:expr) => {{
        $expr;
        return Ok(false);
    }};
}

type ShouldExit = bool;

enum Mode {
    Checkout,
    Add,
}

struct UI {
    mode: Mode,

    change_branches_widget: ChangeBranchesWidget,
    add_branches_widget: AddBranchWidget,
}

impl UI {
    fn new(project: &Project, all_branches: Vec<String>, cur_branch: String) -> UI {
        let saved_branches = project
            .branches
            .iter()
            .map(|b| b.name.clone())
            .collect::<Vec<String>>();

        UI {
            mode: Mode::Checkout,
            change_branches_widget: ChangeBranchesWidget::new(
                project.path.clone(),
                saved_branches.clone(),
                cur_branch,
            ),
            add_branches_widget: AddBranchWidget::new(project.path.clone(), all_branches.clone()),
        }
    }

    fn on_char(&mut self, c: char) -> Result<ShouldExit> {
        match self.mode {
            Mode::Add => continue_after!(self.add_branches_widget.input_char(c)),
            Mode::Checkout => match self.change_branches_widget.mode {
                ChangeBranchesWidgetMode::Search => {
                    continue_after!(self.change_branches_widget.input_char(c))
                }
                ChangeBranchesWidgetMode::Normal => match c {
                    'q' => Ok(true),
                    'a' => continue_after!(self.mode = Mode::Add),
                    '?' => continue_after!(
                        self.change_branches_widget.mode = ChangeBranchesWidgetMode::Search
                    ),
                    'j' => self.on_down(),
                    'k' => self.on_up(),
                    'J' => continue_after!(self.change_branches_widget.swap_down()?),
                    'K' => continue_after!(self.change_branches_widget.swap_up()?),
                    'r' => continue_after!(self.change_branches_widget.remove_selected()?),
                    _ => Ok(false),
                },
            },
        }
    }

    fn on_backspace(&mut self) -> Result<ShouldExit> {
        match self.mode {
            Mode::Add => self.add_branches_widget.remove_char(),
            Mode::Checkout => self.change_branches_widget.remove_char(),
        }
        Ok(false)
    }

    fn on_enter(&mut self) -> Result<ShouldExit> {
        match self.mode {
            Mode::Add => {
                self.add_branches_widget.add_branch()?;
                self.change_branches_widget.reload_saved_branches()?;
                self.mode = Mode::Checkout;
                Ok(false)
            }
            Mode::Checkout => {
                self.change_branches_widget.checkout_selected()?;
                Ok(true)
            }
        }
    }

    fn on_esc(&mut self) -> Result<ShouldExit> {
        match self.mode {
            Mode::Add => match self.add_branches_widget.exit_context() {
                ExitContextResult::Exit => {
                    self.mode = Mode::Checkout;
                }
                ExitContextResult::Continue => {}
            },
            Mode::Checkout => match self.change_branches_widget.mode {
                ChangeBranchesWidgetMode::Normal => return Ok(false),
                ChangeBranchesWidgetMode::Search => {
                    self.change_branches_widget.mode = ChangeBranchesWidgetMode::Normal;
                    self.change_branches_widget.clear_input();
                }
            },
        }

        Ok(false)
    }

    fn on_up(&mut self) -> Result<ShouldExit> {
        match self.mode {
            Mode::Add => self.add_branches_widget.previous(),
            Mode::Checkout => self.change_branches_widget.previous(),
        }
        Ok(false)
    }

    fn on_down(&mut self) -> Result<ShouldExit> {
        match self.mode {
            Mode::Add => self.add_branches_widget.next(),
            Mode::Checkout => self.change_branches_widget.next(),
        }
        Ok(false)
    }

    fn on_shift_up(&mut self) -> Result<ShouldExit> {
        if let Mode::Checkout = self.mode {
            if let ChangeBranchesWidgetMode::Normal = self.change_branches_widget.mode {
                continue_after!(self.change_branches_widget.swap_up()?);
            }
        }
        Ok(false)
    }

    fn on_shift_down(&mut self) -> Result<ShouldExit> {
        if let Mode::Checkout = self.mode {
            if let ChangeBranchesWidgetMode::Normal = self.change_branches_widget.mode {
                continue_after!(self.change_branches_widget.swap_down()?);
            }
        }
        Ok(false)
    }
}

pub fn start_ui(project: Project, branches: Vec<String>, cur_branch: String) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = UI::new(&project, branches, cur_branch);
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
            match handle_input(&mut app) {
                Ok(true) => return Ok(()),
                Err(err) => bail!(err),
                Ok(false) => {} // continue running
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn handle_input(app: &mut UI) -> Result<ShouldExit> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            return if key.modifiers == crossterm::event::KeyModifiers::SHIFT {
                match key.code {
                    KeyCode::Up => app.on_shift_up(),
                    KeyCode::Down => app.on_shift_down(),
                    KeyCode::Char(c) => app.on_char(c),
                    _ => Ok(false),
                }
            } else {
                match key.code {
                    KeyCode::Esc => app.on_esc(),
                    KeyCode::Enter => app.on_enter(),
                    KeyCode::Char(c) => app.on_char(c),
                    KeyCode::Backspace => app.on_backspace(),
                    KeyCode::Down => app.on_down(),
                    KeyCode::Up => app.on_up(),
                    _ => Ok(false),
                }
            };
        }
    }
    Ok(false)
}

fn draw(f: &mut Frame, app: &mut UI) {
    let screen = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size())[0];

    match app.mode {
        Mode::Add => app.add_branches_widget.draw(f, screen),
        Mode::Checkout => app.change_branches_widget.draw(f, screen),
    }
}
