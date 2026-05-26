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
    pub city: Option<String>,
    pub isp: Option<String>,
    pub org: Option<String>,
    pub as_number: Option<String>,
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
fn parse_json_whois(response: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) {
    #[derive(Deserialize)]
    struct WhoisResponse {
        city: Option<String>,
        country: Option<String>,
        isp: Option<String>,
        org: Option<String>,
        asn: Option<String>,
    }

    if let Ok(parsed) = serde_json::from_str::<WhoisResponse>(response) {
        let as_number = parsed.asn.map(|a| format!("AS{}", a));
        return (parsed.city, parsed.country, parsed.isp, parsed.org, as_number);
    }

    (None, None, None, None, None)
}

/// Parse whois response (plain text format)
fn parse_whois(response: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) {
    let mut country = None;
    let mut city = None;
    let mut isp = None;
    let as_number = None;

    for line in response.lines() {
        let line_lower = line.to_lowercase();
        
        if country.is_none() && (line_lower.contains("country") || line_lower.contains("descr")) {
            if let Some(idx) = line.find(':') {
                let value = line[idx + 1..].trim().to_string();
                if !value.is_empty() && country.is_none() {
                    country = Some(value);
                }
            }
        }
        
        if city.is_none() && line_lower.contains("city") {
            if let Some(idx) = line.find(':') {
                let value = line[idx + 1..].trim().to_string();
                if !value.is_empty() {
                    city = Some(value);
                }
            }
        }
        
        if isp.is_none() && (line_lower.contains("isp") || line_lower.contains("org")) {
            if let Some(idx) = line.find(':') {
                let value = line[idx + 1..].trim().to_string();
                if !value.is_empty() && isp.is_none() {
                    isp = Some(value);
                }
            }
        }
    }

    (country, city, isp, None, as_number)
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
                    let (city, country, isp, org, as_number) = parse_json_whois(&whois_text);
                    let ip_info = ExternalIpInfo {
                        ip: Some(ip.clone()),
                        city,
                        country,
                        isp,
                        org,
                        as_number,
                        error: None,
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
        city: whois_info.as_ref().and_then(|i| i.city.clone()),
        isp: whois_info.as_ref().and_then(|i| i.isp.clone()),
        org: whois_info.as_ref().and_then(|i| i.org.clone()),
        as_number: whois_info.as_ref().and_then(|i| i.as_number.clone()),
        error,
    }
}

/// Spawn background task for external IP monitoring
pub fn spawn_external_ip_monitor(
    config: ExternalIpConfig,
    tx: tokio::sync::mpsc::Sender<ExternalIpInfo>,
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
            interval.tick().await;
            
            match update_external_ip_info(&config).await {
                info if info.ip.is_some() => {
                    let _ = tx.send(info).await;
                }
                _ => {}
            }
        }
    });
}
