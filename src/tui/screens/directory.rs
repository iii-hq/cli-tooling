//! Directory input screen

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::PathBuf;

/// State for directory input screen
pub struct DirectoryScreen {
    pub input: String,
    pub cursor_position: usize,
    pub current_dir: PathBuf,
    pub error: Option<String>,
}

impl DirectoryScreen {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            input: String::new(),
            cursor_position: 0,
            current_dir,
            error: None,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += 1;
        self.error = None;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input.remove(self.cursor_position);
            self.error = None;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
        }
    }

    /// Get the resolved path (empty input means current directory)
    pub fn get_path(&self) -> PathBuf {
        if self.input.is_empty() || self.input == "." {
            self.current_dir.clone()
        } else {
            let path = PathBuf::from(&self.input);
            if path.is_absolute() {
                path
            } else {
                self.current_dir.join(path)
            }
        }
    }

    /// Validate the path and return error if invalid
    pub fn validate(&mut self) -> bool {
        let path = self.get_path();

        // Check if parent directory exists (we'll create the target dir)
        if let Some(parent) = path.parent() {
            if !parent.exists() && parent != std::path::Path::new("") {
                self.error = Some(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                ));
                return false;
            }
        }

        // Check if path exists and is not empty
        if path.exists() && path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                let count = entries.count();
                if count > 0 {
                    // Warning but not an error - we'll overwrite
                    self.error = Some(format!(
                        "Warning: Directory exists with {} items (files may be overwritten)",
                        count
                    ));
                    // Still return true to allow continuing
                }
            }
        }

        true
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Current directory info
                Constraint::Length(3), // Input
                Constraint::Length(3), // Error/Warning
                Constraint::Min(4),    // Preview
                Constraint::Length(2), // Help
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Select Directory")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(title, chunks[0]);

        // Current directory info
        let info = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Current directory: ", Style::default().fg(Color::Gray)),
                Span::raw(self.current_dir.display().to_string()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter for current directory, or type a path",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("Info"));
        frame.render_widget(info, chunks[1]);

        // Input field
        let input_display = if self.input.is_empty() {
            Span::styled(". (current directory)", Style::default().fg(Color::DarkGray))
        } else {
            Span::raw(&self.input)
        };

        let input = Paragraph::new(Line::from(vec![Span::raw("> "), input_display]))
            .block(Block::default().borders(Borders::ALL).title("Directory"));
        frame.render_widget(input, chunks[2]);

        // Set cursor position
        frame.set_cursor_position((
            chunks[2].x + 3 + self.cursor_position as u16,
            chunks[2].y + 1,
        ));

        // Error/Warning
        if let Some(error) = &self.error {
            let is_warning = error.starts_with("Warning");
            let color = if is_warning { Color::Yellow } else { Color::Red };
            let error_widget = Paragraph::new(error.as_str())
                .style(Style::default().fg(color))
                .block(Block::default().borders(Borders::NONE));
            frame.render_widget(error_widget, chunks[3]);
        }

        // Preview resolved path
        let resolved = self.get_path();
        let preview = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Will create project at: ", Style::default().fg(Color::Gray)),
            ]),
            Line::from(Span::styled(
                resolved.display().to_string(),
                Style::default().fg(Color::Green),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("Preview"));
        frame.render_widget(preview, chunks[4]);

        // Help
        let help = Paragraph::new("Enter Confirm • Esc Back • q Quit")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[5]);
    }
}
