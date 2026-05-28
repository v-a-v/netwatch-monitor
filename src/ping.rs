use chrono::Utc;

use tokio::io::AsyncBufReadExt;

/// Result of a single ping attempt
#[derive(Debug, Clone)]
pub struct PingResult {
    #[allow(dead_code)]
    pub timestamp: chrono::DateTime<Utc>,
    pub success: bool,
    pub latency_ms: Option<f64>,
    pub ttl: Option<u32>,
    pub dns_error: Option<String>,
}

/// Statistics for a server over a time window
#[derive(Debug, Clone, Default)]
pub struct PingStats {
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub packet_loss_percent: f64,
    pub total_pings: usize,
    pub successful_pings: usize,
    pub ttl: Option<u32>,
    pub dns_error: Option<String>,
}

impl PingStats {
    pub fn from_results(results: &[PingResult]) -> Self {
        if results.is_empty() {
            return Self::default();
        }

        let successful: Vec<&PingResult> = results.iter().filter(|r| r.success).collect();
        let total = results.len();
        let successful_count = successful.len();

        let (min_ms, avg_ms, max_ms) = if successful.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let latencies: Vec<f64> = successful
                .iter()
                .filter_map(|r| r.latency_ms)
                .collect();
            
            let min = latencies.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = latencies.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
            
            (min, avg, max)
        };

        let ttl = successful.first().and_then(|r| r.ttl);
        
        // Get DNS error from first failed result
        let dns_error = results.iter()
            .find(|r| !r.success && r.dns_error.is_some())
            .and_then(|r| r.dns_error.clone());

        let packet_loss = if total > 0 {
            ((total - successful_count) as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            min_ms,
            avg_ms,
            max_ms,
            packet_loss_percent: packet_loss,
            total_pings: total,
            successful_pings: successful_count,
            ttl,
            dns_error,
        }
    }
}

/// Perform a single ping to a host
pub async fn ping_host(host: String, timeout_ms: u64) -> PingResult {
    let timestamp = chrono::Utc::now();

    // Use std::process for cross-platform ping
    let result = tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        let output = std::process::Command::new("ping")
            .args(["-n", "1", "-w", &timeout_ms.to_string(), &host])
            .output();

        #[cfg(not(target_os = "windows"))]
        let output = std::process::Command::new("ping")
            .args(["-c", "1", "-W", &timeout_ms.to_string(), &host])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                PingResult {
                    timestamp,
                    success: true,
                    latency_ms: extract_latency(&stdout),
                    ttl: extract_ttl(&stdout),
                    dns_error: None,
                }
            }
            Ok(out) => {
                // Check for DNS resolution errors
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stdout = String::from_utf8_lossy(&out.stdout);
                let combined = format!("{} {}", stdout, stderr).to_lowercase();
                
                let dns_error = if combined.contains("unknown") 
                    || combined.contains("could not find")
                    || combined.contains("nodename nor servname")
                    || combined.contains("non-existent")
                    || combined.contains("неизвестное")
                    || combined.contains("unknown host")
                    || combined.contains("no address associated")
                {
                    Some("DNS resolution failed".to_string())
                } else {
                    None
                };
                
                PingResult {
                    timestamp,
                    success: false,
                    latency_ms: None,
                    ttl: None,
                    dns_error,
                }
            }
            Err(_) => PingResult {
                timestamp,
                success: false,
                latency_ms: None,
                ttl: None,
                dns_error: None,
            },
        }
    })
    .await
    .unwrap_or(PingResult {
        timestamp,
        success: false,
        latency_ms: None,
        ttl: None,
        dns_error: None,
    });

    result
}

/// Run continuous ping and yield results via channel
pub async fn ping_host_continuous(
    host: String,
    timeout_ms: u64,
    tx: tokio::sync::mpsc::Sender<String>,
    mut stop_rx: tokio::sync::mpsc::Receiver<()>,
) {
    use std::collections::VecDeque;
        let mut lines_buf: VecDeque<String> = VecDeque::new();
    let max_lines = 50;
    
    let timeout_str = timeout_ms.to_string();
    
    #[cfg(target_os = "windows")]
    let args = vec!["-t", "-w", timeout_str.as_str(), host.as_str()];
    
    #[cfg(not(target_os = "windows"))]
    let args = vec!["-W", timeout_str.as_str(), host.as_str()];

    // Spawn ping process
    #[cfg(target_os = "windows")]
    let child = tokio::process::Command::new("ping")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
                .spawn();

    #[cfg(not(target_os = "windows"))]
    let child = tokio::process::Command::new("ping")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
                .spawn();

    if let Ok(mut child) = child {
        if let Some(stdout) = child.stdout.take() {
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                        break;
                    }
                    line = reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                lines_buf.push_back(l);
                                while lines_buf.len() > max_lines {
                                    lines_buf.pop_front();
                                }
                                if let Some(last) = lines_buf.back() {
                                    let _ = tx.send(last.clone()).await;
                                }
                            }
                            Ok(None) => {
                                let _ = child.wait().await;
                                break;
                            }
                            Err(_) => {
                                let _ = child.kill().await;
                                let _ = child.wait().await;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extract latency from ping output (works for Linux/macOS/Windows)
fn extract_latency(output: &str) -> Option<f64> {
    // Linux format: time=0.123 ms
    if let Some(idx) = output.find("time=") {
        let rest = &output[idx + 5..];
        if let Some(end) = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != ' ') {
            if let Ok(val) = rest[..end].trim().parse::<f64>() {
                return Some(val);
            }
        }
    }

    // Windows format: time=123ms
    if let Some(idx) = output.find("time=") {
        let rest = &output[idx + 5..];
        if let Some(end) = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != ' ') {
            if let Ok(val) = rest[..end].trim().parse::<f64>() {
                return Some(val);
            }
        }
    }

    // macOS format: round-trip min/avg/max/stddev = 0.123/0.456/0.789/0.012 ms
    if let Some(idx) = output.find("min/avg/max/") {
        let rest = &output[idx + 12..];
        if let Some(end) = rest.find('/') {
            if let Ok(val) = rest[..end].trim().parse::<f64>() {
                return Some(val);
            }
        }
    }

    None
}

/// Extract TTL from ping output
fn extract_ttl(output: &str) -> Option<u32> {
    // Linux/macOS format: ttl=116
    if let Some(idx) = output.find("ttl=") {
        let rest = &output[idx + 4..];
        if let Some(end) = rest.find(|c: char| !c.is_ascii_digit()) {
            if let Ok(val) = rest[..end].trim().parse::<u32>() {
                return Some(val);
            }
        }
    }

    // Windows format: TTL=116
    if let Some(idx) = output.find("TTL=") {
        let rest = &output[idx + 4..];
        if let Some(end) = rest.find(|c: char| !c.is_ascii_digit()) {
            if let Ok(val) = rest[..end].trim().parse::<u32>() {
                return Some(val);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_latency_linux() {
        let output = "PING 8.8.8.8 (8.8.8.8) 56(84) bytes of data.\n64 bytes from 8.8.8.8: icmp_seq=1 ttl=116 time=12.3 ms";
        assert!(extract_latency(output).is_some());
    }

    #[test]
    fn test_extract_latency_windows() {
        let output = "Reply from 8.8.8.8: bytes=32 time=15ms TTL=116";
        assert!(extract_latency(output).is_some());
    }
}
