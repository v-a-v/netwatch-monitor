use anyhow::Result;
use serde::Deserialize;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};
use crate::config::ExternalIpConfig;

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
    pub last_update: Option<String>,
    pub error: Option<String>,
}

fn parse_plain_ip(response: &str) -> Option<String> {
    let ip = response.trim();
    if ip.is_empty() || ip.len() > 45 {
        None
    } else {
        Some(ip.to_string())
    }
}

fn parse_json_ip(response: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct IpResp { ip: Option<String> }
    serde_json::from_str::<IpResp>(response).ok()?.ip
}

fn parse_json_whois(response: &str) -> ExternalIpInfo {
    #[derive(Deserialize, Default)]
    struct Connection { asn: Option<u32>, isp: Option<String>, org: Option<String> }

    #[derive(Deserialize, Default)]
    struct TimeZone { id: Option<String> }

    #[derive(Deserialize)]
    struct Whois {
        success: Option<bool>,
        city: Option<String>,
        region: Option<String>,
        country: Option<String>,
        country_code: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        #[serde(default)] connection: Connection,
        #[serde(default)] timezone: TimeZone,
    }

    let parsed = match serde_json::from_str::<Whois>(response) {
        Ok(p) => p,
        Err(_) => return ExternalIpInfo::default(),
    };

    if parsed.success == Some(false) {
        return ExternalIpInfo::default();
    }

    ExternalIpInfo {
        ip: None,
        city: parsed.city,
        region: parsed.region,
        country: parsed.country,
        country_code: parsed.country_code,
        isp: parsed.connection.isp,
        org: parsed.connection.org,
        as_number: parsed.connection.asn.map(|x| format!("AS{}", x)),
        latitude: parsed.latitude,
        longitude: parsed.longitude,
        timezone: parsed.timezone.id,
        last_update: Some(chrono::Local::now().format("%H:%M:%S").to_string()),
        error: None,
    }
}

async fn fetch_self_ip() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("NetWatch-Monitor/1.0")
        .build()
        .ok()?;
    let resp = client.get("https://ifconfig.io/ip").send().await.ok()?;
    let txt = resp.text().await.ok()?;
    parse_plain_ip(&txt)
}

async fn fetch_ipwho(ip: &str) -> Option<ExternalIpInfo> {
    let url = format!("https://ipwho.is/{}", ip);
    let body = reqwest::get(&url).await.ok()?.text().await.ok()?;
    let mut info = parse_json_whois(&body);
    info.ip = Some(ip.to_string());
    Some(info)
}

async fn fetch_ipapi(ip: &str) -> Option<ExternalIpInfo> {
    #[derive(Deserialize)]
    struct Api {
        city: Option<String>, region: Option<String>, country_name: Option<String>, country_code: Option<String>,
        asn: Option<String>, org: Option<String>, timezone: Option<String>, latitude: Option<f64>, longitude: Option<f64>,
    }
    let url = format!("https://ipapi.co/{}/json/", ip);
    let body = reqwest::get(&url).await.ok()?.text().await.ok()?;
    let p: Api = serde_json::from_str(&body).ok()?;
    Some(ExternalIpInfo {
        ip: Some(ip.to_string()), city: p.city, region: p.region,
        country: p.country_name, country_code: p.country_code,
        isp: None, org: p.org, as_number: p.asn,
        latitude: p.latitude, longitude: p.longitude,
        timezone: p.timezone,
        last_update: Some(chrono::Local::now().format("%H:%M:%S").to_string()), error: None,
    })
}

async fn fetch_ipinfo(ip: &str) -> Option<ExternalIpInfo> {
    #[derive(Deserialize)]
    struct Info { city: Option<String>, region: Option<String>, country: Option<String>, org: Option<String>, timezone: Option<String>, loc: Option<String> }
    let url = format!("https://ipinfo.io/{}/json", ip);
    let body = reqwest::get(&url).await.ok()?.text().await.ok()?;
    let p: Info = serde_json::from_str(&body).ok()?;

    let (lat, lon) = if let Some(loc) = p.loc {
        let mut s = loc.split(',');
        (s.next()?.parse().ok(), s.next()?.parse().ok())
    } else { (None, None) };

    Some(ExternalIpInfo {
        ip: Some(ip.to_string()),
        city: p.city, region: p.region, country: p.country.clone(), country_code: None,
        isp: None, org: p.org, as_number: None,
        latitude: lat, longitude: lon,
        timezone: p.timezone,
        last_update: Some(chrono::Local::now().format("%H:%M:%S").to_string()),
        error: None,
    })
}

pub async fn update_external_ip_info(_cfg: &ExternalIpConfig) -> ExternalIpInfo {
    let ip = match fetch_self_ip().await {
        Some(ip) => ip,
        None => return ExternalIpInfo { ip: None, error: Some("Cannot fetch IP".into()), ..Default::default() },
    };

    if let Some(info) = fetch_ipwho(&ip).await {
        if info.country.is_some() { return info; }
    }
    if let Some(info) = fetch_ipapi(&ip).await {
        if info.country.is_some() || info.city.is_some() { return info; }
    }
    if let Some(info) = fetch_ipinfo(&ip).await { return info; }

    ExternalIpInfo {
        ip: Some(ip),
        error: Some("WHOIS lookup failed".into()),
        last_update: Some(chrono::Local::now().format("%H:%M:%S").to_string()),
        ..Default::default()
    }
}

pub fn spawn_external_ip_monitor(
    config: ExternalIpConfig,
    tx: tokio::sync::mpsc::Sender<ExternalIpInfo>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(config.check_interval_sec));

        let info = update_external_ip_info(&config).await;
        let _ = tx.send(info).await;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => break,
                _ = timer.tick() => {
                    let info = update_external_ip_info(&config).await;
                    let _ = tx.send(info).await;
                }
            }
        }
    });
}