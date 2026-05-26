mod config;
mod external_ip;
mod ping;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use external_ip::ExternalIpInfo;
use ping::{ping_host, PingResult};
use ui::render;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup tracing - disabled for TUI cleanliness, logs go to stderr only
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "netwatch_monitor=warn,tower::buffer=error".into()),
        )
        .with(tracing_subscriber::fmt::layer()
            .with_target(false)
            .without_time()
            .with_writer(std::io::stderr))
        .init();

    // Load config
    let config = Config::load_or_default()?;
    info!("Loaded config with {} servers", config.servers.len());

    if config.servers.is_empty() {
        eprintln!("No servers configured. Please add servers to config.toml");
        std::process::exit(1);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channels for communication
    let (tx, mut rx) = mpsc::channel::<(usize, PingResult)>(100);
    let (ip_tx, mut ip_rx) = mpsc::channel::<ExternalIpInfo>(5);

    // Spawn external IP monitor
    external_ip::spawn_external_ip_monitor(config.external_ip.clone(), ip_tx);

    // Spawn ping tasks for each server
    let mut ping_handles = vec![];
    for (idx, server) in config.servers.iter().enumerate() {
        let tx_clone = tx.clone();
        let host = server.host.clone();
        let timeout = server.timeout_ms;
        let interval = Duration::from_secs(config.interval);

        let handle = tokio::spawn(async move {
            loop {
                let result = ping_host(host.clone(), timeout).await;
                let _ = tx_clone.send((idx, result)).await;
                tokio::time::sleep(interval).await;
            }
        });
        ping_handles.push(handle);
    }

    // Initialize results storage
    let mut results: Vec<VecDeque<PingResult>> = vec![VecDeque::new(); config.servers.len()];
    let history_size = config.history_size;
    let mut selected_server: usize = 0;
    let mut external_ip_info: Option<ExternalIpInfo> = None;
    let mut running = true;

    // Main loop
    while running {
        // Process ping results
        while let Ok((server_idx, ping_result)) = rx.try_recv() {
            if let Some(server_results) = results.get_mut(server_idx) {
                server_results.push_back(ping_result);
                // Keep only the last N results
                while server_results.len() > history_size {
                    server_results.pop_front();
                }
            }
        }

        // Process external IP updates
        while let Ok(ip_info) = ip_rx.try_recv() {
            external_ip_info = Some(ip_info);
        }

        // Handle user input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => running = false,
                        KeyCode::Char('r') => {
                            // Manual refresh - results will come from the ping tasks
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if selected_server > 0 {
                                selected_server -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if selected_server < config.servers.len() - 1 {
                                selected_server += 1;
                            }
                        }
                        KeyCode::Home => {
                            selected_server = 0;
                        }
                        KeyCode::End => {
                            selected_server = config.servers.len() - 1;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Convert VecDeque to Vec for rendering
        let results_vec: Vec<Vec<PingResult>> = results
            .iter()
            .map(|dq| dq.iter().cloned().collect())
            .collect();

        // Render UI
        terminal.draw(|frame| {
            render(frame, &config.servers, &results_vec, selected_server, external_ip_info.as_ref());
        })?;
    }

    // Cleanup
    drop(tx);
    for handle in ping_handles {
        handle.abort();
    }

    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    disable_raw_mode()?;

    info!("NetWatch Monitor stopped");
    Ok(())
}
