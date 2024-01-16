use anyhow::{anyhow, Result};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, List, ListState, Paragraph},
    Frame,
};

use crate::core;

pub struct StatefulList<T> {
    pub state: ListState, // TODO: Make private
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.state.select(Some(0));
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.state.select(None);
    }

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn items(&self) -> &Vec<T> {
        &self.items
    }
}

pub enum ExitContextResult {
    Exit,
    Continue,
}

pub struct AddBranchWidget {
    project_path: String,
    all_branches: Vec<String>,
    add_branch_input: String,
    add_branch_autocomplete: StatefulList<String>,
}

impl AddBranchWidget {
    pub fn new(project_path: String, all_branches: Vec<String>) -> AddBranchWidget {
        AddBranchWidget {
            project_path,
            all_branches: all_branches.clone(),
            add_branch_input: String::new(),
            add_branch_autocomplete: StatefulList::with_items(all_branches),
        }
    }

    pub fn add_branch(&mut self) -> Result<()> {
        let new_branch = self.get_branch_name();
        if new_branch.is_empty() {
            return Ok(());
        }
        core::add_branch(self.project_path.as_str(), new_branch)?;
        Ok(())
    }

    pub fn update_autocomplete(&mut self) {
        let items = self
            .all_branches
            .iter()
            .filter(|b| b.starts_with(self.add_branch_input.as_str()))
            .cloned()
            .collect::<Vec<String>>();
        self.add_branch_autocomplete.set_items(items);
        self.add_branch_autocomplete.state.select(None)
    }

    pub fn input_char(&mut self, c: char) {
        self.add_branch_input.push(c);
        self.update_autocomplete();
    }

    pub fn remove_char(&mut self) {
        self.add_branch_input.pop();
        self.update_autocomplete();
    }

    pub fn exit_context(&mut self) -> ExitContextResult {
        match self.add_branch_autocomplete.state.selected() {
            Some(_) => {
                self.add_branch_autocomplete.state.select(None);
                ExitContextResult::Continue
            }
            None => {
                self.clear();
                ExitContextResult::Exit
            }
        }
    }

    pub fn clear(&mut self) {
        self.add_branch_input.clear();
        self.update_autocomplete();
    }

    pub fn get_branch_name(&self) -> String {
        match self.add_branch_autocomplete.state.selected() {
            Some(i) => self.add_branch_autocomplete.items[i].clone(),
            None => self.add_branch_input.clone(),
        }
    }

    pub fn next(&mut self) {
        self.add_branch_autocomplete.next();
    }

    pub fn previous(&mut self) {
        self.add_branch_autocomplete.previous();
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
            .split(area);

        let input = Paragraph::new(self.add_branch_input.as_str())
            .block(Block::default().title("Add branch").borders(Borders::ALL));

        f.render_widget(input, chunks[0]);

        let items = self
            .add_branch_autocomplete
            .items
            .iter()
            .map(|b| Text::raw(b.as_str()))
            .collect::<Vec<Text>>();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Branches"))
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[1], &mut self.add_branch_autocomplete.state);
    }
}

pub struct ChangeBranchesWidget {
    project_path: String,
    saved_branches: StatefulList<String>,
}

impl ChangeBranchesWidget {
    pub fn new(project_path: String, saved_branches: Vec<String>) -> ChangeBranchesWidget {
        ChangeBranchesWidget {
            project_path,
            saved_branches: StatefulList::with_items(saved_branches),
        }
    }

    pub fn next(&mut self) {
        self.saved_branches.next();
    }

    pub fn previous(&mut self) {
        self.saved_branches.previous();
    }

    pub fn checkout_selected(&self) -> Result<()> {
        let selected = self
            .saved_branches
            .selected()
            .ok_or(anyhow!("no branch selected"))?;
        let branch = self.saved_branches.items()[selected].as_str();
        core::checkout(self.project_path.as_str(), branch)?;
        Ok(())
    }

    pub fn remove_selected(&mut self) -> Result<()> {
        let selected = self
            .saved_branches
            .selected()
            .ok_or(anyhow!("no branch selected"))?;
        let branch = self.saved_branches.items()[selected].clone();
        core::remove_branch(self.project_path.as_str(), branch)?;
        self.reload_saved_branches()?;
        Ok(())
    }

    pub fn reload_saved_branches(&mut self) -> Result<()> {
        self.saved_branches = StatefulList::with_items(
            core::get_branches(self.project_path.as_str())?
                .iter()
                .map(|b| b.name.clone())
                .collect::<Vec<String>>(),
        );
        Ok(())
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
            .split(area);

        let input = Paragraph::new(self.project_path.as_str()).block(
            Block::default()
                .title("Change branches")
                .borders(Borders::ALL),
        );

        f.render_widget(input, chunks[0]);

        let items = self
            .saved_branches
            .items
            .iter()
            .map(|b| Text::raw(b.as_str()))
            .collect::<Vec<Text>>();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Branches"))
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[1], &mut self.saved_branches.state);
    }
}