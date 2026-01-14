//! iii installation check screen

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Options for iii installation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IiiOption {
    OpenDocs,
    AutoInstall,
    Skip,
}

impl IiiOption {
    pub fn label(&self) -> &'static str {
        match self {
            IiiOption::OpenDocs => "Open documentation (https://iii.sh)",
            IiiOption::AutoInstall => "Automatically install iii",
            IiiOption::Skip => "Skip and continue without iii",
        }
    }

    pub fn all() -> Vec<IiiOption> {
        vec![IiiOption::OpenDocs, IiiOption::AutoInstall, IiiOption::Skip]
    }
}

/// State for the iii check screen
pub struct IiiCheckScreen {
    pub iii_installed: bool,
    pub iii_version: Option<String>,
    pub selected: usize,
    pub list_state: ListState,
}

impl IiiCheckScreen {
    pub fn new(installed: bool, version: Option<String>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            iii_installed: installed,
            iii_version: version,
            selected: 0,
            list_state,
        }
    }

    pub fn next(&mut self) {
        let options = IiiOption::all();
        self.selected = (self.selected + 1) % options.len();
        self.list_state.select(Some(self.selected));
    }

    pub fn previous(&mut self) {
        let options = IiiOption::all();
        self.selected = if self.selected == 0 {
            options.len() - 1
        } else {
            self.selected - 1
        };
        self.list_state.select(Some(self.selected));
    }

    pub fn selected_option(&self) -> IiiOption {
        IiiOption::all()[self.selected]
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Status
                Constraint::Min(8),    // Options
                Constraint::Length(2), // Help
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Motia CLI Setup")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(title, chunks[0]);

        // Status
        let status_text = if self.iii_installed {
            let version = self.iii_version.as_deref().unwrap_or("unknown");
            vec![
                Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Green)),
                    Span::raw("iii is installed"),
                ]),
                Line::from(vec![
                    Span::raw("  Version: "),
                    Span::styled(version, Style::default().fg(Color::Yellow)),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(Color::Red)),
                    Span::raw("iii is not installed"),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "iii is required to run Motia applications.",
                    Style::default().fg(Color::Gray),
                )),
            ]
        };

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, chunks[1]);

        // Options (only show if not installed)
        if !self.iii_installed {
            let items: Vec<ListItem> = IiiOption::all()
                .iter()
                .enumerate()
                .map(|(i, opt)| {
                    let style = if i == self.selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    let prefix = if i == self.selected { "▶ " } else { "  " };
                    ListItem::new(format!("{}{}", prefix, opt.label())).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Options"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            frame.render_stateful_widget(list, chunks[2], &mut self.list_state);
        } else {
            let continue_text = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Press Enter to continue...",
                    Style::default().fg(Color::Green),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title("Continue"));
            frame.render_widget(continue_text, chunks[2]);
        }

        // Help
        let help = Paragraph::new("↑↓ Navigate • Enter Select • q Quit")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[3]);
    }
}
