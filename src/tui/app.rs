//! Main TUI application

use crate::config::generator;
use crate::runtime::{check, iii};
use crate::templates::{copier, fetcher::TemplateFetcher, fetcher::TemplateSource, version};
use crate::tui::screens::{
    directory::DirectoryScreen, iii_check::IiiCheckScreen, iii_check::IiiOption,
    language_select::LanguageSelectScreen, summary::SummaryScreen,
    template_select::TemplateSelectScreen,
};
use crate::{Args, CLI_VERSION};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Current screen in the TUI flow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    IiiCheck,
    TemplateSelect,
    DirectoryInput,
    LanguageSelect,
    Summary,
}

/// Main application state
struct App {
    current_screen: Screen,
    iii_screen: IiiCheckScreen,
    template_screen: TemplateSelectScreen,
    directory_screen: DirectoryScreen,
    language_screen: Option<LanguageSelectScreen>,
    summary_screen: Option<SummaryScreen>,
    fetcher: TemplateFetcher,
    should_quit: bool,
}

impl App {
    fn new(args: &Args) -> Self {
        // Determine template source
        let source = match &args.template_dir {
            Some(path) => TemplateSource::local(path.clone()),
            None => TemplateSource::default_remote(),
        };

        // Check iii installation
        let iii_installed = iii::is_installed();
        let iii_version = if iii_installed {
            iii::get_version()
        } else {
            None
        };

        Self {
            current_screen: Screen::IiiCheck,
            iii_screen: IiiCheckScreen::new(iii_installed, iii_version),
            template_screen: TemplateSelectScreen::new(),
            directory_screen: DirectoryScreen::new(),
            language_screen: None,
            summary_screen: None,
            fetcher: TemplateFetcher::new(source),
            should_quit: false,
        }
    }

    async fn load_templates(&mut self) {
        match self.fetcher.fetch_root_manifest().await {
            Ok(root_manifest) => {
                let mut templates = Vec::new();

                for template_name in &root_manifest.templates {
                    match self.fetcher.fetch_template_manifest(template_name).await {
                        Ok(manifest) => {
                            templates.push((template_name.clone(), manifest));
                        }
                        Err(e) => {
                            self.template_screen
                                .set_error(format!("Failed to load template '{}': {}", template_name, e));
                            return;
                        }
                    }
                }

                self.template_screen.set_templates(templates);
            }
            Err(e) => {
                self.template_screen
                    .set_error(format!("Failed to load templates: {}", e));
            }
        }
    }

    async fn handle_key_event(&mut self, key: KeyCode) {
        // Global quit
        if key == KeyCode::Char('q') {
            self.should_quit = true;
            return;
        }

        match self.current_screen {
            Screen::IiiCheck => self.handle_iii_check_key(key).await,
            Screen::TemplateSelect => self.handle_template_select_key(key),
            Screen::DirectoryInput => self.handle_directory_input_key(key),
            Screen::LanguageSelect => self.handle_language_select_key(key).await,
            Screen::Summary => self.handle_summary_key(key),
        }
    }

    async fn handle_iii_check_key(&mut self, key: KeyCode) {
        if self.iii_screen.iii_installed {
            // iii is installed, just continue on Enter
            if key == KeyCode::Enter {
                self.current_screen = Screen::TemplateSelect;
                self.load_templates().await;
            }
            return;
        }

        match key {
            KeyCode::Up => self.iii_screen.previous(),
            KeyCode::Down => self.iii_screen.next(),
            KeyCode::Enter => {
                match self.iii_screen.selected_option() {
                    IiiOption::OpenDocs => {
                        let _ = iii::open_docs();
                    }
                    IiiOption::AutoInstall => {
                        // Try to install iii
                        if iii::install().await.is_ok() {
                            self.iii_screen.iii_installed = true;
                            self.iii_screen.iii_version = iii::get_version();
                        }
                    }
                    IiiOption::Skip => {
                        // Continue without iii
                        self.current_screen = Screen::TemplateSelect;
                        self.load_templates().await;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_template_select_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => self.template_screen.previous(),
            KeyCode::Down => self.template_screen.next(),
            KeyCode::Enter => {
                if self.template_screen.selected_template().is_some() {
                    self.current_screen = Screen::DirectoryInput;
                }
            }
            _ => {}
        }
    }

    fn handle_directory_input_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => self.directory_screen.insert_char(c),
            KeyCode::Backspace => self.directory_screen.delete_char(),
            KeyCode::Left => self.directory_screen.move_cursor_left(),
            KeyCode::Right => self.directory_screen.move_cursor_right(),
            KeyCode::Esc => {
                self.current_screen = Screen::TemplateSelect;
            }
            KeyCode::Enter => {
                if self.directory_screen.validate() {
                    // Initialize language screen with template requirements
                    if let Some((_, manifest)) = self.template_screen.selected_template() {
                        self.language_screen = Some(LanguageSelectScreen::new(manifest));
                        self.current_screen = Screen::LanguageSelect;
                    }
                }
            }
            _ => {}
        }
    }

    async fn handle_language_select_key(&mut self, key: KeyCode) {
        let language_screen = match &mut self.language_screen {
            Some(screen) => screen,
            None => return,
        };

        match key {
            KeyCode::Up => language_screen.previous(),
            KeyCode::Down => language_screen.next(),
            KeyCode::Char(' ') => language_screen.toggle(),
            KeyCode::Esc => {
                self.current_screen = Screen::DirectoryInput;
            }
            KeyCode::Enter => {
                let selected_languages = language_screen.selected_languages();

                // Check runtimes
                match check::check_runtimes(&selected_languages) {
                    Ok(_runtimes) => {
                        // Proceed to setup
                        self.setup_project(selected_languages).await;
                    }
                    Err(e) => {
                        // Show error - missing runtimes
                        let (_, manifest) = self.template_screen.selected_template().unwrap();
                        let project_dir = self.directory_screen.get_path();
                        let mut summary = SummaryScreen::new(
                            manifest.name.clone(),
                            &project_dir,
                            selected_languages,
                        );
                        summary.set_error(e.to_string());
                        self.summary_screen = Some(summary);
                        self.current_screen = Screen::Summary;
                    }
                }
            }
            _ => {}
        }
    }

    async fn setup_project(&mut self, selected_languages: Vec<check::Language>) {
        let (template_name, manifest) = self.template_screen.selected_template().unwrap();
        let project_dir = self.directory_screen.get_path();

        let mut summary = SummaryScreen::new(
            manifest.name.clone(),
            &project_dir,
            selected_languages.clone(),
        );

        // Check version compatibility
        if let Some(warning) = version::check_compatibility(CLI_VERSION, &manifest.version) {
            summary.set_version_warning(warning);
        }

        self.summary_screen = Some(summary);
        self.current_screen = Screen::Summary;

        // Copy template files
        match copier::copy_template(
            &self.fetcher,
            template_name,
            manifest,
            &project_dir,
            &selected_languages,
        )
        .await
        {
            Ok(copied_files) => {
                // Generate iii config
                if let Err(e) = generator::write_config(&project_dir, &selected_languages).await {
                    if let Some(summary) = &mut self.summary_screen {
                        summary.set_error(format!("Failed to write iii config: {}", e));
                    }
                    return;
                }

                if let Some(summary) = &mut self.summary_screen {
                    summary.set_complete(copied_files);
                }
            }
            Err(e) => {
                if let Some(summary) = &mut self.summary_screen {
                    summary.set_error(format!("Failed to copy template: {}", e));
                }
            }
        }
    }

    fn handle_summary_key(&mut self, key: KeyCode) {
        if let Some(summary) = &self.summary_screen {
            if summary.is_complete || summary.error.is_some() {
                if key == KeyCode::Enter || key == KeyCode::Char('q') {
                    self.should_quit = true;
                }
            }
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        match self.current_screen {
            Screen::IiiCheck => self.iii_screen.render(frame, area),
            Screen::TemplateSelect => self.template_screen.render(frame, area),
            Screen::DirectoryInput => self.directory_screen.render(frame, area),
            Screen::LanguageSelect => {
                if let Some(screen) = &mut self.language_screen {
                    screen.render(frame, area);
                }
            }
            Screen::Summary => {
                if let Some(screen) = &self.summary_screen {
                    screen.render(frame, area);
                }
            }
        }
    }
}

/// Run the TUI application
pub async fn run(args: Args) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(&args);

    // Main loop
    loop {
        terminal.draw(|frame| app.render(frame))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key_event(key.code).await;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print final summary if project was created
    if let Some(summary) = &app.summary_screen {
        if summary.is_complete {
            println!("\n✓ Project created successfully!\n");
            println!("Next steps:");
            for (i, step) in summary.get_next_steps().iter().enumerate() {
                println!("  {}. {}", i + 1, step);
            }
            println!();
        }
    }

    Ok(())
}
