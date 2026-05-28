use anyhow::Result;
use serde::Deserialize;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

use crate::config::ExternalIpConfig;

/// External IP information
#[derive(Debug, Clone, Default)]
pub struct ExternalIpInfo {
    pub ip: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub isp: Option<String>,
    pub org: Option<String>,
    pub as_number: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
    #[allow(dead_code)]
    pub error: Option<String>,
}

/// Parse plain IP response
fn parse_plain_ip(response: &str) -> Option<String> {
    let cleaned = response.trim();
    // Validate basic IPv4/IPv6 format
    if cleaned.is_empty() || cleaned.len() > 45 {
        return None;
    }
    Some(cleaned.to_string())
}

/// Parse JSON response from ipwho.is
fn parse_json_ip(response: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct IpResponse {
        ip: Option<String>,
    }

    if let Ok(parsed) = serde_json::from_str::<IpResponse>(response) {
        return parsed.ip;
    }

    None
}

/// Parse JSON whois response from ipwho.is
fn parse_json_whois(response: &str) -> ExternalIpInfo {
    #[derive(Deserialize)]
    struct WhoisResponse {
        city: Option<String>,
        country: Option<String>,
        country_code: Option<String>,
        region: Option<String>,
        isp: Option<String>,
        org: Option<String>,
        asn: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        timezone: Option<String>,
    }

    if let Ok(parsed) = serde_json::from_str::<WhoisResponse>(response) {
        let as_number = parsed.asn.map(|a| format!("AS{}", a));
        return ExternalIpInfo {
            ip: None,
            city: parsed.city,
            country: parsed.country,
            country_code: parsed.country_code,
            region: parsed.region,
            isp: parsed.isp,
            org: parsed.org,
            as_number,
            latitude: parsed.latitude,
            longitude: parsed.longitude,
            timezone: parsed.timezone,
            error: None,
        };
    }

    ExternalIpInfo::default()
}

/// Fetch external IP from endpoint
pub async fn fetch_external_ip(endpoint: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("NetWatch-Monitor/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Try JSON parsing first
    if let Some(ip) = parse_json_ip(&body) {
        return Ok(ip);
    }

    // Fall back to plain text
    if let Some(ip) = parse_plain_ip(&body) {
        return Ok(ip);
    }

    Err("Failed to parse IP from response".to_string())
}

/// Fetch whois information
pub async fn fetch_whois_info(endpoint: &str, ip: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("NetWatch-Monitor/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!("{}{}", endpoint.trim_end_matches('/'), ip);
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
}

/// Update external IP info
pub async fn update_external_ip_info(config: &ExternalIpConfig) -> ExternalIpInfo {
    debug!("Fetching external IP from {}", config.endpoint);

    // Fetch IP
    let ip_result = fetch_external_ip(&config.endpoint).await;
    
    let (ip, whois_info, error) = match ip_result {
        Ok(ip) => {
            info!("External IP: {}", ip);

            // Fetch whois info from ipwho.is
            let whois_url = format!("https://ipwho.is/{}", ip);
            let whois_result = fetch_whois_info(&whois_url, "").await;

            match whois_result {
                Ok(whois_text) => {
                    let ip_info = parse_json_whois(&whois_text);
                    let ip_info = ExternalIpInfo {
                        ip: Some(ip.clone()),
                        ..ip_info
                    };
                    (Some(ip), Some(ip_info), None)
                }
                Err(e) => {
                    debug!("Failed to fetch whois: {}", e);
                    (Some(ip), None, None)
                }
            }
        }
        Err(ref e) => {
            error!("Failed to fetch external IP: {}", e);
            (None, None, Some(e.clone()))
        }
    };

    ExternalIpInfo {
        ip,
        country: whois_info.as_ref().and_then(|i| i.country.clone()),
        country_code: whois_info.as_ref().and_then(|i| i.country_code.clone()),
        city: whois_info.as_ref().and_then(|i| i.city.clone()),
        region: whois_info.as_ref().and_then(|i| i.region.clone()),
        isp: whois_info.as_ref().and_then(|i| i.isp.clone()),
        org: whois_info.as_ref().and_then(|i| i.org.clone()),
        as_number: whois_info.as_ref().and_then(|i| i.as_number.clone()),
        latitude: whois_info.as_ref().and_then(|i| i.latitude),
        longitude: whois_info.as_ref().and_then(|i| i.longitude),
        timezone: whois_info.as_ref().and_then(|i| i.timezone.clone()),
        error,
    }
}

/// Spawn background task for external IP monitoring
pub fn spawn_external_ip_monitor(
    config: ExternalIpConfig,
    tx: tokio::sync::mpsc::Sender<ExternalIpInfo>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(config.check_interval_sec));
        
        // Initial fetch
        match update_external_ip_info(&config).await {
            info if info.ip.is_some() => {
                let _ = tx.send(info).await;
            }
            _ => {}
        }

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }
                _ = interval.tick() => {
                    match update_external_ip_info(&config).await {
                        info if info.ip.is_some() => {
                            let _ = tx.send(info).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}
