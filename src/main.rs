use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io::{self, Write};
use std::process::Command;

mod storage;
use storage::Storage;

#[derive(Parser)]
#[command(name = "bcmd", about = "Manage your terminal commands as shortcuts")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a saved shortcut command
    Run { shortcut: String },
    /// Create a new shortcut directly via CLI
    Create { shortcut: String, command: String },
    /// Delete a shortcut directly via CLI
    Delete { shortcut: String },
    /// List all shortcuts compactly in the terminal
    List,
}

#[derive(PartialEq)]
enum AppState {
    MainList,
    DeleteConfirm,
    Help,
}

fn execute_command(cmd_str: &str) {
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let flag = if cfg!(target_os = "windows") { "/C" } else { "-c" };
    
    println!("\n--- Executing: {} ---", cmd_str);
    let mut child = Command::new(shell)
        .arg(flag)
        .arg(cmd_str)
        .spawn()
        .expect("Failed to execute process");
    let _ = child.wait();
    
    println!("\nPress Enter to return to menu...");
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut storage = Storage::load();
    match cli.command {
        Some(Commands::Run { shortcut }) => {
            if let Some(cmd) = storage.shortcuts.get(&shortcut) {
                execute_command(cmd);
            } else {
                eprintln!("ERROR: Shortcut '{}' not found.", shortcut);
            }
        }
        Some(Commands::Create { shortcut, command }) => {
            storage.shortcuts.insert(shortcut.clone(), command);
            storage.save()?;
            println!("SUCCESS: Shortcut '{}' created successfully!", shortcut);
        }
        Some(Commands::Delete { shortcut }) => {
            if storage.shortcuts.remove(&shortcut).is_some() {
                storage.save()?;
                println!("DELETED: Shortcut '{}' removed.", shortcut);
            } else {
                println!("WARNING: Shortcut '{}' not found.", shortcut);
            }
        }
        Some(Commands::List) => {
            if storage.shortcuts.is_empty() {
                println!("No shortcuts saved yet.");
            } else {
                for (k, v) in &storage.shortcuts {
                    println!("{}: {}", k, v);
                }
            }
        }
        None => {
            loop {
                let action = run_tui(&mut storage)?;
                if action == "quit" {
                    break;
                }
            }
        }
    }
    Ok(())
}
fn run_tui(storage: &mut Storage) -> Result<String, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let mut app_state = AppState::MainList;
    let mut key_to_delete = String::new();
    let return_action;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(size);

            let items: Vec<ListItem> = storage.shortcuts.iter().map(|(k, v)| {
                let key = Span::styled(format!(" {:<10} ", k), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
                let val = Span::styled(v, Style::default().fg(Color::White));
                ListItem::new(Line::from(vec![key, Span::styled(" -> ", Style::default().fg(Color::DarkGray)), val]))
            }).collect();

            let list_widget = List::new(items)
                .block(Block::default().title(" My CLI Shortcuts ".magenta().bold()).borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(100, 100, 255))))
                .highlight_style(Style::default().bg(Color::Rgb(40, 40, 80)).add_modifier(Modifier::BOLD));
            f.render_stateful_widget(list_widget, chunks[0], &mut list_state);

            let help_text = match app_state {
                AppState::MainList => " [Up/Down] Navigate | [Enter] Run | [c] Create | [d] Delete | [h] Help | [q] Quit",
                AppState::DeleteConfirm => " Are you sure? [y] Yes | [n] No / Cancel",
                AppState::Help => " [Esc/q] Close Help",
            };
            f.render_widget(Paragraph::new(help_text).block(Block::default().title(" Controls ".yellow()).borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))), chunks[1]);
            if app_state == AppState::DeleteConfirm {
                let v_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(40), Constraint::Length(7), Constraint::Percentage(40)]).split(size);
                let h_chunks = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(20), Constraint::Percentage(60), Constraint::Percentage(20)]).split(v_chunks[1]);
                let area = h_chunks[1];
                f.render_widget(Clear, area);
                let msg = format!("\n  Delete shortcut '{}'? [y/n]", key_to_delete);
                f.render_widget(Paragraph::new(msg).block(Block::default().title(" Confirmation ".red().bold()).borders(Borders::ALL).border_style(Style::default().fg(Color::Red))), area);
            }

            if app_state == AppState::Help {
                let v_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(25), Constraint::Length(10), Constraint::Percentage(25)]).split(size);
                let h_chunks = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(20), Constraint::Percentage(60), Constraint::Percentage(20)]).split(v_chunks[1]);
                let area = h_chunks[1];
                f.render_widget(Clear, area);
                let help_content = vec![
                    Line::from(vec!["bcmd - Bookmark CLI Commands".magenta().bold()]),
                    Line::from("  c     -> Open creation input"),
                    Line::from("  d     -> Delete selected shortcut"),
                    Line::from("  Enter -> Run selected shortcut instantly"),
                ];
                f.render_widget(Paragraph::new(help_content).block(Block::default().title(" Quick Help ".yellow()).borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow))), area);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match app_state {
                AppState::MainList => match key.code {
                    KeyCode::Char('q') => { return_action = String::from("quit"); break; }
                    KeyCode::Char('h') => app_state = AppState::Help,
                    KeyCode::Char('c') => { return_action = String::from("create"); break; }
                    KeyCode::Up => { let i = match list_state.selected() { Some(i) => if i == 0 { storage.shortcuts.len().saturating_sub(1) } else { i - 1 }, None => 0 }; list_state.select(Some(i)); }
                    KeyCode::Down => { let i = match list_state.selected() { Some(i) => if i >= storage.shortcuts.len().saturating_sub(1) { 0 } else { i + 1 }, None => 0 }; list_state.select(Some(i)); }
                    KeyCode::Char('d') => { if let Some(index) = list_state.selected() { if let Some(k) = storage.shortcuts.keys().nth(index).cloned() { key_to_delete = k; app_state = AppState::DeleteConfirm; } } }
                    KeyCode::Enter => {
                        if let Some(index) = list_state.selected() {
                            if let Some(cmd) = storage.shortcuts.values().nth(index).cloned() {
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                terminal.show_cursor()?;
                                execute_command(&cmd);
                                return_action = String::from("reload");
                                break;
                            }
                        }
                    }
                    _ => {}
                },
                AppState::DeleteConfirm => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => { storage.shortcuts.remove(&key_to_delete); storage.save()?; app_state = AppState::MainList; }
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => app_state = AppState::MainList,
                    _ => {}
                },
                AppState::Help => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app_state = AppState::MainList,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if return_action == "create" {
        print!("\nEnter Shortcut Key (e.g. gs): ");
        io::stdout().flush()?;
        let mut shortcut = String::new();
        io::stdin().read_line(&mut shortcut)?;
        
        print!("Enter Full Command (e.g. git status): ");
        io::stdout().flush()?;
        let mut command = String::new();
        io::stdin().read_line(&mut command)?;

        let sc_trimmed = shortcut.trim().to_string();
        let cmd_trimmed = command.trim().to_string();

        if !sc_trimmed.is_empty() && !cmd_trimmed.is_empty() {
            storage.shortcuts.insert(sc_trimmed, cmd_trimmed);
            storage.save()?;
        }
        return Ok(String::from("reload"));
    }

    Ok(return_action)
}
