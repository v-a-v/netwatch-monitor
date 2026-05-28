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

/// Render the main TUI
pub fn render(
    frame: &mut Frame,
    servers: &[ServerConfig],
    results: &[Vec<PingResult>],
    selected: usize,
    external_ip: Option<&ExternalIpInfo>,
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
    render_server_list(frame, chunks[1], servers, results, selected);
    render_stats(frame, chunks[2], &results[selected]);
    render_help(frame, chunks[3]);
}

/// HEADER BLOCK
fn render_header(frame: &mut Frame, area: Rect, external_ip: Option<&ExternalIpInfo>) {
    let time = chrono::Local::now().format("%H:%M:%S");

    let ip_info = match external_ip {
        Some(info) if info.ip.is_some() => {
            let ip = info.ip.clone().unwrap();

            let mut parts: Vec<String> = Vec::new();
            if let Some(v) = &info.city { parts.push(v.clone()); }
            if let Some(v) = &info.region { parts.push(v.clone()); }
            if let Some(country) = &info.country {
                let cc = info.country_code.clone().unwrap_or_default();
                if cc.is_empty() { parts.push(country.clone()); }
                else { parts.push(format!("{} [{}]", country, cc)); }
            }
            if let Some(v) = &info.as_number { parts.push(v.clone()); }
            if let Some(v) = &info.isp { parts.push(v.clone()); }
            if let Some(v) = &info.org { parts.push(v.clone()); }
            if let Some(v) = &info.timezone { parts.push(v.clone()); }
            if let (Some(lat), Some(lon)) = (info.latitude, info.longitude) {
                parts.push(format!("{:.0}°N {:.0}°E", lat, lon));
            }

            let updated = info.last_update.clone().unwrap_or("??:??:??".into());

            let mut s = format!("🌍 {} ({}) • {}", ip, parts.join(", "), updated);

            // dynamic width
            let max_width = area.width.saturating_sub(6) as usize;

            if s.len() > max_width {
                let mut compact = Vec::<String>::new();
                if let Some(v) = &info.city { compact.push(v.clone()); }
                if let Some(v) = &info.country_code { compact.push(v.clone()); }
                if let Some(v) = &info.as_number { compact.push(v.clone()); }

                s = format!("🌍 {} ({}) • {}", ip, compact.join(", "), updated);
            }

            if s.len() > max_width {
                let cc = info.country_code.clone().unwrap_or_default();
                let asn = info.as_number.clone().unwrap_or_default();
                s = format!("🌍 {} ({}, {}) • {}", ip, cc, asn, updated);
            }

            s
        }
        _ => "🌍 Detecting...".into(),
    };

    let header = Paragraph::new(
        format!("🌐 NetWatch │ {} │ {}", time, ip_info)
    )
    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(header, area);
}

/// SERVER LIST
fn render_server_list(
    frame: &mut Frame,
    area: Rect,
    servers: &[ServerConfig],
    results: &[Vec<PingResult>],
    selected: usize,
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
                "NO DATA".into()
            };

            let ttl = stats.ttl.map(|t| t.to_string()).unwrap_or_else(|| "--".into());

            let avg_color = match stats.avg_ms {
                x if x < 50.0 => Color::Green,
                x if x < 100.0 => Color::Yellow,
                x if x < 200.0 => Color::LightRed,
                _ => Color::Red,
            };

            let status_color = if stats.dns_error.is_some() {
                Color::Red
            } else if stats.successful_pings > 0 {
                if stats.packet_loss_percent > 50.0 {
                    Color::Red
                } else if stats.packet_loss_percent > 20.0 {
                    Color::Yellow
                } else {
                    Color::Green
                }
            } else {
                Color::Gray
            };

            Row::new(vec![
                Cell::from(format!("{} {}",
                    if i == selected { "▶" } else { " " }, server.name)),
                Cell::from(Span::styled(server.host.clone(), Style::default().fg(Color::Gray))),
                Cell::from(Span::styled(format!("{:.1}ms", stats.avg_ms), Style::default().fg(avg_color))),
                Cell::from(ttl),
                Cell::from(Span::styled(status, Style::default().fg(status_color))),
                Cell::from(render_history_bar(&results[i])),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(25),
            Constraint::Min(30),
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Min(20),
            Constraint::Min(40),
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
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default().borders(Borders::ALL).title("Servers")
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

/// HISTORY BAR
fn render_history_bar(results: &[PingResult]) -> String {
    if results.is_empty() {
        return " ".repeat(40);
    }

    let recent: Vec<&PingResult> = results.iter().rev().take(40).collect();
    let mut bar = String::new();

    for r in recent.iter().rev() {
        let symbol = if r.success {
            match r.latency_ms {
                Some(x) if x < 50.0 => "█",
                Some(x) if x < 100.0 => "▓",
                Some(x) if x < 200.0 => "▒",
                _ => "░",
            }
        } else {
            "✗"
        };
        bar.push_str(symbol);
    }

    bar
}

/// STATISTICS PANEL
fn render_stats(frame: &mut Frame, area: Rect, results: &[PingResult]) {
    let stats = PingStats::from_results(results);

    let status_color = if stats.packet_loss_percent > 50.0 {
        Color::Red
    } else if stats.packet_loss_percent > 20.0 { Color::Yellow }
    else if stats.successful_pings > 0 { Color::Green }
    else { Color::Gray };

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

/// HELP FOOTER
fn render_help(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new("↑/↓: Select | Enter: Detail | Esc/q: Quit | r: Refresh")
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(help, area);
}

/// PING DETAIL VIEW
pub fn render_ping_detail(
    frame: &mut Frame,
    server_name: &str,
    host: &str,
    ping_output: &str,
) {
    let size = frame.area();

    let layout = Layout::default()
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

    frame.render_widget(header, layout[0]);

    let lines: Vec<&str> = ping_output.lines().collect();
    let total = lines.len() as u16;
    let available = layout[1].height.saturating_sub(2);
    let offset = if total > available { total - available } else { 0 };

    let output = Paragraph::new(ping_output.to_string())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Live Ping Output")
                .border_style(Style::default().fg(Color::Green)),
        )
        .scroll((offset, 0));

    frame.render_widget(output, layout[1]);

    let help = Paragraph::new("Esc: Back | q: Quit")
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(help, layout[2]);
}
