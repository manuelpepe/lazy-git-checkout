use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, List, ListState, Paragraph},
    Frame,
};

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
    all_branches: Vec<String>,
    add_branch_input: String,
    add_branch_autocomplete: StatefulList<String>,
}

impl AddBranchWidget {
    pub fn new(all_branches: Vec<String>) -> AddBranchWidget {
        AddBranchWidget {
            all_branches: all_branches.clone(),
            add_branch_input: String::new(),
            add_branch_autocomplete: StatefulList::with_items(all_branches),
        }
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
