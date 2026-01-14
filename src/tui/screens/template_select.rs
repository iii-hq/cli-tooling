//! Template selection screen

use crate::templates::manifest::TemplateManifest;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// State for template selection screen
pub struct TemplateSelectScreen {
    pub templates: Vec<(String, TemplateManifest)>,
    pub selected: usize,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
}

impl TemplateSelectScreen {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            templates: Vec::new(),
            selected: 0,
            list_state,
            loading: true,
            error: None,
        }
    }

    pub fn set_templates(&mut self, templates: Vec<(String, TemplateManifest)>) {
        self.templates = templates;
        self.loading = false;
        if !self.templates.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.loading = false;
    }

    pub fn next(&mut self) {
        if self.templates.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.templates.len();
        self.list_state.select(Some(self.selected));
    }

    pub fn previous(&mut self) {
        if self.templates.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.templates.len() - 1
        } else {
            self.selected - 1
        };
        self.list_state.select(Some(self.selected));
    }

    pub fn selected_template(&self) -> Option<&(String, TemplateManifest)> {
        self.templates.get(self.selected)
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Template list
                Constraint::Length(6), // Description
                Constraint::Length(2), // Help
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Select Template")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(title, chunks[0]);

        // Loading or error state
        if self.loading {
            let loading = Paragraph::new("Loading templates...")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Templates"));
            frame.render_widget(loading, chunks[1]);
            return;
        }

        if let Some(error) = &self.error {
            let error_text = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"));
            frame.render_widget(error_text, chunks[1]);
            return;
        }

        // Template list
        let items: Vec<ListItem> = self
            .templates
            .iter()
            .enumerate()
            .map(|(i, (_name, manifest))| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let prefix = if i == self.selected { "▶ " } else { "  " };
                let version = format!(" (v{})", manifest.version);
                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(&manifest.name, style),
                    Span::styled(version, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Templates"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Description of selected template
        if let Some((_, manifest)) = self.selected_template() {
            let desc_lines = vec![
                Line::from(vec![
                    Span::styled("Description: ", Style::default().fg(Color::Gray)),
                    Span::raw(&manifest.description),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Required: ", Style::default().fg(Color::Gray)),
                    Span::raw(manifest.requires.join(", ")),
                ]),
                Line::from(vec![
                    Span::styled("Optional: ", Style::default().fg(Color::Gray)),
                    Span::raw(manifest.optional.join(", ")),
                ]),
            ];

            let desc = Paragraph::new(desc_lines)
                .block(Block::default().borders(Borders::ALL).title("Details"));
            frame.render_widget(desc, chunks[2]);
        }

        // Help
        let help = Paragraph::new("↑↓ Navigate • Enter Select • q Quit")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[3]);
    }
}
