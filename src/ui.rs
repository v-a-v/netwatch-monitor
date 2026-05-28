use crate::config::ServerConfig;
use crate::external_ip::ExternalIpInfo;
use crate::ping::{PingResult, PingStats};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

/// Render the main UI
pub fn render(
    frame: &mut Frame,
    servers: &[ServerConfig],
    results: &[Vec<PingResult>],
    selected: usize,
    external_ip: Option<&ExternalIpInfo>,
    tick: u8,
) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Length(3),
        ])
        .split(size);

    render_header(frame, chunks[0], external_ip);
    render_server_list(frame, chunks[1], servers, results, selected, tick);
    render_stats(frame, chunks[2], &results[selected]);
    render_help(frame, chunks[3]);
}

fn render_header(frame: &mut Frame, area: Rect, external_ip: Option<&ExternalIpInfo>) {
    let now = chrono::Local::now();
    let datetime = now.format("%Y-%m-%d %H:%M:%S");

    let ip_info = match external_ip {
        Some(info) => {
            if let Some(ref ip) = info.ip {
                let location = match (&info.city, &info.country) {
                    (Some(city), Some(country)) => format!("{}, {}", city, country),
                    (Some(city), None) => city.clone(),
                    (None, Some(country)) => country.clone(),
                    _ => "Unknown".to_string(),
                };
                format!("🌍 {} ({})", ip, location)
            } else {
                "🌍 Loading...".to_string()
            }
        }
        None => "🌍 Detecting...".to_string(),
    };

    let header_text = format!(
        "🌐 NetWatch Monitor  │  {}  │  {}",
        datetime, ip_info
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    frame.render_widget(header, area);
}

fn render_server_list(
    frame: &mut Frame,
    area: Rect,
    servers: &[ServerConfig],
    results: &[Vec<PingResult>],
    selected: usize,
    _tick: u8,
) {
    let rows: Vec<Row> = servers
        .iter()
        .enumerate()
        .map(|(i, server)| {
            let stats = PingStats::from_results(&results[i]);

            let status = if let Some(ref dns_err) = stats.dns_error {
                format!("🚫 {}", dns_err)
            } else if stats.successful_pings > 0 {
                if stats.packet_loss_percent > 50.0 {
                    format!("🔴 {:.1}% loss", stats.packet_loss_percent)
                } else if stats.packet_loss_percent > 20.0 {
                    format!("🟠 {:.1}% loss", stats.packet_loss_percent)
                } else {
                    format!("🟢 {:.1}% loss", stats.packet_loss_percent)
                }
            } else {
                "NO DATA".to_string()
            };

            let ttl = stats.ttl.map(|t| t.to_string()).unwrap_or_else(|| "--".to_string());
            let history = render_history_bar(&results[i]);

            let avg_color = if let Some(_) = stats.dns_error {
                Color::Gray
            } else if stats.avg_ms < 50.0 {
                Color::Green
            } else if stats.avg_ms < 100.0 {
                Color::Yellow
            } else if stats.avg_ms < 200.0 {
                Color::LightRed
            } else {
                Color::Red
            };

            Row::new(vec![
                Cell::from(format!(
                    "{} {}",
                    if i == selected { "▶" } else { " " },
                    server.name
                )),
                Cell::from(Span::styled(server.host.clone(), Style::default().fg(Color::Gray))),
                Cell::from(Span::styled(format!("{:.1}ms", stats.avg_ms), Style::default().fg(avg_color))),
                Cell::from(ttl),
                Cell::from(Span::styled(status, Style::default().fg(avg_color))),
                Cell::from(history),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(22),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Min(15),
            Constraint::Min(30),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from("Server"),
            Cell::from("Host"),
            Cell::from("Avg (ms)"),
            Cell::from("Hop"),
            Cell::from("Status"),
            Cell::from("History"),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Servers")
            .border_style(Style::default().fg(Color::Blue)),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    frame.render_widget(table, area);
}

fn render_history_bar(results: &[PingResult]) -> String {
    if results.is_empty() {
        return " ".repeat(40);
    }

    let recent: Vec<&PingResult> = results.iter().rev().take(40).collect();
    let mut bar = String::new();

    for result in recent.iter().rev() {
        let color = if result.success {
            match result.latency_ms {
                Some(lat) if lat < 50.0 => "█",
                Some(lat) if lat < 100.0 => "▓",
                Some(lat) if lat < 200.0 => "▒",
                _ => "░",
            }
        } else {
            "✗"
        };
        bar.push_str(color);
    }

    bar
}

fn render_stats(frame: &mut Frame, area: Rect, results: &[PingResult]) {
    let stats = PingStats::from_results(results);

    let status_color = if stats.packet_loss_percent > 50.0 {
        Color::Red
    } else if stats.packet_loss_percent > 20.0 {
        Color::Yellow
    } else if stats.successful_pings > 0 {
        Color::Green
    } else {
        Color::Gray
    };

    let stats_text = vec![
        Line::from(vec![
            Span::styled("Min: ", Style::default().fg(Color::White)),
            Span::styled(format!("{:.2}ms ", stats.min_ms), Style::default().fg(Color::Cyan)),
            Span::styled("Avg: ", Style::default().fg(Color::White)),
            Span::styled(format!("{:.2}ms ", stats.avg_ms), Style::default().fg(Color::Cyan)),
            Span::styled("Max: ", Style::default().fg(Color::White)),
            Span::styled(format!("{:.2}ms", stats.max_ms), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Packet Loss: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.1}% ", stats.packet_loss_percent),
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Success: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{}/{} ", stats.successful_pings, stats.total_pings),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(Span::styled(
            "Legend: █ <50ms ▓ <100ms ▒ <200ms ░ >200ms ✗ fail",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let stats_paragraph = Paragraph::new(stats_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Statistics")
            .border_style(Style::default().fg(Color::Green)),
    );

    frame.render_widget(stats_paragraph, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new("↑/↓: Select server | q: Quit | r: Refresh now")
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(help, area);
}

pub fn render_ping_detail(
    frame: &mut Frame,
    server_name: &str,
    host: &str,
    ping_output: &str,
) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(size);

    let header = Paragraph::new(format!("🔍 Ping: {} ({})", server_name, host))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(header, chunks[0]);

    let lines: Vec<&str> = ping_output.lines().collect();
    let total = lines.len() as u16;
    let available = chunks[1].height.saturating_sub(2);
    let offset = if total > available {
        total - available
    } else {
        0
    };

    let output = Paragraph::new(ping_output)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Live Ping Output")
                .border_style(Style::default().fg(Color::Green)),
        )
        .scroll((offset, 0));

    frame.render_widget(output, chunks[1]);

    let help = Paragraph::new("Esc: Back | q: Quit")
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(help, chunks[2]);
}
