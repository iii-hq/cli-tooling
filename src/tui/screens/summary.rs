//! Summary and completion screen

use crate::runtime::check::Language;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::Path;

/// State for the summary screen
pub struct SummaryScreen {
    pub template_name: String,
    pub project_dir: String,
    pub selected_languages: Vec<Language>,
    pub copied_files: Vec<String>,
    pub version_warning: Option<String>,
    pub is_complete: bool,
    pub error: Option<String>,
}

impl SummaryScreen {
    pub fn new(
        template_name: String,
        project_dir: &Path,
        selected_languages: Vec<Language>,
    ) -> Self {
        Self {
            template_name,
            project_dir: project_dir.display().to_string(),
            selected_languages,
            copied_files: Vec::new(),
            version_warning: None,
            is_complete: false,
            error: None,
        }
    }

    pub fn set_complete(&mut self, copied_files: Vec<String>) {
        self.copied_files = copied_files;
        self.is_complete = true;
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn set_version_warning(&mut self, warning: String) {
        self.version_warning = Some(warning);
    }

    /// Get the next steps based on selected languages
    pub fn get_next_steps(&self) -> Vec<String> {
        let mut steps = Vec::new();

        let has_js_ts = self
            .selected_languages
            .iter()
            .any(|l| matches!(l, Language::TypeScript | Language::JavaScript));

        let has_python = self.selected_languages.contains(&Language::Python);

        steps.push(format!("cd {}", self.project_dir));

        if has_js_ts {
            steps.push("npm install".to_string());
        }

        if has_python {
            steps.push("python3 -m venv .venv".to_string());
            steps.push("source .venv/bin/activate".to_string());
            steps.push("pip install -r requirements".to_string());
        }

        if has_js_ts {
            steps.push("npm run build".to_string());
        }

        steps.push("iii start".to_string());

        steps
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(8),  // Project info
                Constraint::Length(4),  // Version warning (if any)
                Constraint::Min(10),    // Next steps
                Constraint::Length(2),  // Help
            ])
            .split(area);

        // Title
        let title_text = if self.is_complete {
            "✓ Project Created Successfully!"
        } else if self.error.is_some() {
            "✗ Error Creating Project"
        } else {
            "Creating Project..."
        };

        let title_color = if self.is_complete {
            Color::Green
        } else if self.error.is_some() {
            Color::Red
        } else {
            Color::Yellow
        };

        let title = Paragraph::new(title_text)
            .style(Style::default().fg(title_color).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(title, chunks[0]);

        // Project info
        let languages: Vec<&str> = self
            .selected_languages
            .iter()
            .map(|l| l.display_name())
            .collect();

        let info_lines = vec![
            Line::from(vec![
                Span::styled("Template: ", Style::default().fg(Color::Gray)),
                Span::raw(&self.template_name),
            ]),
            Line::from(vec![
                Span::styled("Directory: ", Style::default().fg(Color::Gray)),
                Span::raw(&self.project_dir),
            ]),
            Line::from(vec![
                Span::styled("Languages: ", Style::default().fg(Color::Gray)),
                Span::raw(languages.join(", ")),
            ]),
            Line::from(vec![
                Span::styled("Files: ", Style::default().fg(Color::Gray)),
                Span::raw(format!("{} files copied", self.copied_files.len())),
            ]),
        ];

        let info = Paragraph::new(info_lines)
            .block(Block::default().borders(Borders::ALL).title("Project Info"));
        frame.render_widget(info, chunks[1]);

        // Version warning
        if let Some(warning) = &self.version_warning {
            let warning_widget = Paragraph::new(warning.as_str())
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("⚠ Warning"));
            frame.render_widget(warning_widget, chunks[2]);
        } else if let Some(error) = &self.error {
            let error_widget = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"));
            frame.render_widget(error_widget, chunks[2]);
        }

        // Next steps
        if self.is_complete {
            let steps = self.get_next_steps();
            let mut step_lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    "Run the following commands to get started:",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(""),
            ];

            for (i, step) in steps.iter().enumerate() {
                step_lines.push(Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::DarkGray)),
                    Span::styled(step, Style::default().fg(Color::White)),
                ]));
            }

            let next_steps = Paragraph::new(step_lines)
                .block(Block::default().borders(Borders::ALL).title("Next Steps"));
            frame.render_widget(next_steps, chunks[3]);
        } else if !self.is_complete && self.error.is_none() {
            let loading = Paragraph::new("Setting up project files...")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Progress"));
            frame.render_widget(loading, chunks[3]);
        }

        // Help
        let help_text = if self.is_complete {
            "Press q or Enter to exit"
        } else {
            "Please wait..."
        };
        let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[4]);
    }
}
