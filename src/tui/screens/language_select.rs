//! Language selection screen

use crate::runtime::check::Language;
use crate::templates::manifest::TemplateManifest;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Language option with selection state
#[derive(Debug, Clone)]
pub struct LanguageOption {
    pub language: Language,
    pub selected: bool,
    pub required: bool, // Cannot be deselected
}

/// State for language selection screen
pub struct LanguageSelectScreen {
    pub options: Vec<LanguageOption>,
    pub cursor: usize,
    pub list_state: ListState,
}

impl LanguageSelectScreen {
    pub fn new(manifest: &TemplateManifest) -> Self {
        let mut options = Vec::new();

        // Add TypeScript if required or optional
        if manifest.is_required("typescript") || manifest.is_optional("typescript") {
            options.push(LanguageOption {
                language: Language::TypeScript,
                selected: manifest.is_required("typescript"), // Pre-select if required
                required: manifest.is_required("typescript"),
            });
        }

        // Add JavaScript if required or optional
        if manifest.is_required("javascript") || manifest.is_optional("javascript") {
            options.push(LanguageOption {
                language: Language::JavaScript,
                selected: manifest.is_required("javascript"),
                required: manifest.is_required("javascript"),
            });
        }

        // Add Python if required or optional
        if manifest.is_required("python") || manifest.is_optional("python") {
            options.push(LanguageOption {
                language: Language::Python,
                selected: manifest.is_required("python"),
                required: manifest.is_required("python"),
            });
        }

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            options,
            cursor: 0,
            list_state,
        }
    }

    pub fn next(&mut self) {
        if self.options.is_empty() {
            return;
        }
        self.cursor = (self.cursor + 1) % self.options.len();
        self.list_state.select(Some(self.cursor));
    }

    pub fn previous(&mut self) {
        if self.options.is_empty() {
            return;
        }
        self.cursor = if self.cursor == 0 {
            self.options.len() - 1
        } else {
            self.cursor - 1
        };
        self.list_state.select(Some(self.cursor));
    }

    pub fn toggle(&mut self) {
        if let Some(option) = self.options.get_mut(self.cursor) {
            // Cannot deselect required languages
            if !option.required {
                option.selected = !option.selected;
            }
        }
    }

    /// Get all selected languages
    pub fn selected_languages(&self) -> Vec<Language> {
        self.options
            .iter()
            .filter(|o| o.selected)
            .map(|o| o.language)
            .collect()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(4), // Instructions
                Constraint::Min(8),    // Language list
                Constraint::Length(4), // Selected summary
                Constraint::Length(2), // Help
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Select Languages")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(title, chunks[0]);

        // Instructions
        let instructions = Paragraph::new(vec![
            Line::from("Choose which languages to include in your project."),
            Line::from(Span::styled(
                "Required languages cannot be deselected.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("Info"));
        frame.render_widget(instructions, chunks[1]);

        // Language list
        let items: Vec<ListItem> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let checkbox = if opt.selected { "[✓]" } else { "[ ]" };
                let required_tag = if opt.required { " (required)" } else { "" };

                let style = if i == self.cursor {
                    if opt.required {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    }
                } else if opt.required {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };

                let prefix = if i == self.cursor { "▶ " } else { "  " };

                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(checkbox, style),
                    Span::raw(" "),
                    Span::styled(opt.language.display_name(), style),
                    Span::styled(required_tag, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Languages"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, chunks[2], &mut self.list_state);

        // Selected summary
        let selected: Vec<&str> = self
            .options
            .iter()
            .filter(|o| o.selected)
            .map(|o| o.language.display_name())
            .collect();

        let summary_text = if selected.is_empty() {
            Span::styled(
                "No languages selected",
                Style::default().fg(Color::Red),
            )
        } else {
            Span::styled(
                selected.join(", "),
                Style::default().fg(Color::Green),
            )
        };

        let summary = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Selected: ", Style::default().fg(Color::Gray)),
                summary_text,
            ]),
        ])
        .block(Block::default().borders(Borders::ALL).title("Summary"));
        frame.render_widget(summary, chunks[3]);

        // Help
        let help = Paragraph::new("↑↓ Navigate • Space Toggle • Enter Confirm • Esc Back • q Quit")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[4]);
    }
}
