use std::{
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use maxminddb::{Reader, geoip2};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

const GEOIP_CACHE_DIR: &str = "/tmp/rollping";
const GEOIP_DB_FILENAME: &str = "GeoLite2-City.mmdb";
const GEOIP_DB_URL: &str = "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-City.mmdb";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

pub struct GeoIpClient {
    reader: Option<Reader<Vec<u8>>>,
}

impl GeoIpClient {
    pub fn new() -> Self {
        match Self::initialize() {
            Ok(reader) => {
                info!("GeoIP database loaded successfully");
                GeoIpClient {
                    reader: Some(reader),
                }
            }
            Err(e) => {
                warn!(
                    "Failed to initialize GeoIP: {}. Geolocation will be disabled.",
                    e
                );
                GeoIpClient { reader: None }
            }
        }
    }

    fn initialize() -> Result<Reader<Vec<u8>>> {
        let db_path = Self::get_db_path();

        // Try to load existing database
        if db_path.exists() {
            debug!("Loading existing GeoIP database from {:?}", db_path);
            let reader = Reader::open_readfile(&db_path)
                .context("Failed to open existing GeoIP database")?;
            return Ok(reader);
        }

        // Database doesn't exist, try to download it
        info!("GeoIP database not found, downloading from mirror...");
        Self::download_database(&db_path)?;

        let reader =
            Reader::open_readfile(&db_path).context("Failed to open downloaded GeoIP database")?;
        Ok(reader)
    }

    fn get_db_path() -> PathBuf {
        Path::new(GEOIP_CACHE_DIR).join(GEOIP_DB_FILENAME)
    }

    fn download_database(db_path: &Path) -> Result<()> {
        // Create cache directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).context("Failed to create GeoIP cache directory")?;
        }

        debug!("Downloading GeoIP database from {}", GEOIP_DB_URL);

        // Download the database
        let response =
            reqwest::blocking::get(GEOIP_DB_URL).context("Failed to download GeoIP database")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to download GeoIP database: HTTP {}",
                response.status()
            );
        }

        let bytes = response
            .bytes()
            .context("Failed to read GeoIP database response")?;

        // Write to disk
        fs::write(db_path, bytes).context("Failed to write GeoIP database to disk")?;

        info!("GeoIP database downloaded successfully to {:?}", db_path);
        Ok(())
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<Location> {
        let reader = self.reader.as_ref()?;

        match reader.lookup::<geoip2::City>(ip) {
            Ok(Some(city_data)) => {
                debug!(
                    "GeoIP lookup for {}: city={:?}, country={:?}",
                    ip,
                    city_data
                        .city
                        .as_ref()
                        .and_then(|c| c.names.as_ref())
                        .and_then(|n| n.get("en")),
                    city_data
                        .country
                        .as_ref()
                        .and_then(|c| c.names.as_ref())
                        .and_then(|n| n.get("en"))
                );

                let country = city_data
                    .country
                    .as_ref()
                    .and_then(|c| c.names.as_ref())
                    .and_then(|n| n.get("en"))
                    .map(|s| s.to_string());

                let country_code = city_data
                    .country
                    .as_ref()
                    .and_then(|c| c.iso_code)
                    .map(|s| s.to_string());

                let city_name = city_data
                    .city
                    .as_ref()
                    .and_then(|c| c.names.as_ref())
                    .and_then(|n| n.get("en"))
                    .map(|s| s.to_string());

                let (latitude, longitude) = city_data
                    .location
                    .as_ref()
                    .map(|l| (l.latitude, l.longitude))
                    .unwrap_or((None, None));

                Some(Location {
                    country,
                    country_code,
                    city: city_name,
                    latitude,
                    longitude,
                })
            }
            Ok(None) => {
                debug!("GeoIP lookup for {} returned no data", ip);
                None
            }
            Err(e) => {
                debug!("GeoIP lookup failed for {}: {}", ip, e);
                None
            }
        }
    }

    pub fn is_available(&self) -> bool {
        self.reader.is_some()
    }
}

/// Get the public IP address of the current machine
pub fn get_public_ip() -> Result<IpAddr> {
    debug!("Detecting public IP address...");

    // Try multiple services for reliability
    let services = [
        "https://api.ipify.org",
        "https://icanhazip.com",
        "https://ifconfig.me/ip",
    ];

    for service in &services {
        match reqwest::blocking::get(*service) {
            Ok(response) => {
                if let Ok(ip_str) = response.text()
                    && let Ok(ip) = ip_str.trim().parse::<IpAddr>()
                {
                    debug!("Detected public IP: {} (from {})", ip, service);
                    return Ok(ip);
                }
            }
            Err(e) => {
                debug!("Failed to get IP from {}: {}", service, e);
                continue;
            }
        }
    }

    anyhow::bail!("Failed to detect public IP address from any service")
}
