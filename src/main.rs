use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex, LazyLock};

use reqwest::Client;

// Shared mutable state: background task writes, handler reads
static GLOBAL_METRICS: LazyLock<Arc<Mutex<String>>> = LazyLock::new(|| Arc::new(Mutex::new(String::from(""))));

#[derive(Debug, Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    // Replace these fields with actual values from your API documentation
    token: String,
}

#[derive(Debug, Deserialize, Clone)]
struct StatsResponse {
    #[serde(rename = "PLATFORMS")]
    platforms: i64,
    #[serde(rename = "ROMS")]
    roms: i64,
    #[serde(rename = "SAVES")]
    saves: i64,
    #[serde(rename = "STATES")]
    states: i64,
    #[serde(rename = "SCREENSHOTS")]
    screenshots: i64,
    #[serde(rename = "TOTAL_FILESIZE_BYTES")]
    total_filesize_bytes: i64,
}

// Per-ROM data from /api/roms (Option 3 & 4)
#[derive(Debug, Deserialize, Clone)]
struct RomEntry {
    name: String,
    fs_name: String,
    fs_extension: String,
    has_simple_single_file: bool,
    has_nested_single_file: bool,
    has_multiple_files: bool,
    is_easyrpg_game: bool,
    has_manual: bool,
    is_unidentified: bool,
    is_identified: bool,
    missing_from_fs: bool,
    is_physical: bool,
    crc_hash: String,
    md5_hash: String,
    sha1_hash: String,
    ra_hash: String,
}

// Aggregated ROM category counts
#[derive(Debug, Clone)]
struct RomCategories {
    entries: Vec<(String, i64)>,
}

impl Default for RomCategories {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

// Aggregated file structure counts
#[derive(Debug, Clone)]
struct FileStructureCounts {
    entries: Vec<(String, i64)>,
}

impl Default for FileStructureCounts {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

// Aggregated manual counts
#[derive(Debug, Clone)]
struct ManualCounts {
    entries: Vec<(String, i64)>,
}

impl Default for ManualCounts {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

// Aggregated integrity state counts
#[derive(Debug, Clone)]
struct IntegrityCounts {
    entries: Vec<(String, i64)>,
}

impl Default for IntegrityCounts {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

// Aggregated hash coverage counts
#[derive(Debug, Clone)]
struct HashCounts {
    entries: Vec<(String, i64)>,
}

impl Default for HashCounts {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

// Per-platform data from /api/platforms
#[derive(Debug, Deserialize, Clone)]
struct PlatformEntry {
    name: String,
    rom_count: i64,
    firmware_count: i64,
    is_unidentified: bool,
    is_identified: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct PlatformsResponse {
    platforms: Vec<PlatformEntry>,
}

impl PlatformsResponse {
    fn from_array(arr: Vec<PlatformEntry>) -> Self {
        Self { platforms: arr }
    }
}

impl Default for PlatformsResponse {
    fn default() -> Self {
        Self { platforms: Vec::<PlatformEntry>::default() }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct RomPageData {
    items: Vec<RomEntry>,
}

impl Default for StatsResponse {
    fn default() -> Self {
        Self { platforms: Default::default(), roms: Default::default(), saves: Default::default(), states: Default::default(), screenshots: Default::default(), total_filesize_bytes: Default::default() }
    }
}

// NOTE: use Display trait to give Prometheus metrics
impl Display for StatsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = write!(f, "# HELP romm_platforms A metric for number of platforms in ROMM\n");
        let _ = write!(f, "# TYPE romm_platforms gauge\n");
        let _ = write!(f, "romm_platforms {}\n", self.platforms);

        let _ = write!(f, "# HELP romm_roms A metric for number of ROMs in ROMM\n");
        let _ = write!(f, "# TYPE romm_roms gauge\n");
        let _ = write!(f, "romm_roms {}\n", self.roms);

        let _ = write!(f, "# HELP romm_saves A metric for number of saves in ROMM\n");
        let _ = write!(f, "# TYPE romm_saves gauge\n");
        let _ = write!(f, "romm_saves {}\n", self.saves);

        let _ = write!(f, "# HELP romm_states A metric for number of save states in ROMM\n");
        let _ = write!(f, "# TYPE romm_states gauge\n");
        let _ = write!(f, "romm_states {}\n", self.states);

        let _ = write!(f, "# HELP romm_screenshots A metric for number of screenshots in ROMM\n");
        let _ = write!(f, "# TYPE romm_screenshots gauge\n");
        let _ = write!(f, "romm_screenshots {}\n", self.screenshots);

        let _ = write!(f, "# HELP romm_total_filesize_bytes A metric for the total filesize of ROMM\n");
        let _ = write!(f, "# TYPE romm_total_filesize_bytes gauge\n");
        let _ = write!(f, "romm_total_filesize_bytes {}\n", self.total_filesize_bytes);

        Ok(())
    }
}

// Option 1 & 2: Platform-level metrics
impl Display for PlatformsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for entry in &self.platforms {
            // Option 1: Per-platform ROM count
            let _ = write!(f, "# HELP romm_platforms_rom_count Number of ROMs per platform\n");
            let _ = write!(f, "# TYPE romm_platforms_rom_count gauge\n");
            let _ = write!(f, "romm_platforms_rom_count{{platform=\"{}\"}} {}\n", entry.name, entry.rom_count);

            // Option 1: Per-platform firmware count
            let _ = write!(f, "# HELP romm_platforms_firmware_count Number of firmware items per platform\n");
            let _ = write!(f, "# TYPE romm_platforms_firmware_count gauge\n");
            let _ = write!(f, "romm_platforms_firmware_count{{platform=\"{}\"}} {}\n", entry.name, entry.firmware_count);
        }

        // Option 2: Platform identification status (2 gauges: identified vs unidentified)
        let mut unidentified = 0i64;
        let mut identified = 0i64;
        for entry in &self.platforms {
            if entry.is_unidentified {
                unidentified += 1;
            }
            if entry.is_identified {
                identified += 1;
            }
        }
        let _ = write!(f, "# HELP romm_platforms_unidentified_count Number of unidentified platforms\n");
        let _ = write!(f, "# TYPE romm_platforms_unidentified_count gauge\n");
        let _ = write!(f, "romm_platforms_unidentified_count {}\n", unidentified);
        let _ = write!(f, "# HELP romm_platforms_identified_count Number of identified platforms\n");
        let _ = write!(f, "# TYPE romm_platforms_identified_count gauge\n");
        let _ = write!(f, "romm_platforms_identified_count {}\n", identified);

        Ok(())
    }
}

// Option 3: ROM category from name prefix
impl Display for RomCategories {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = write!(f, "# HELP romm_category Number of ROMs per category (parsed from name prefix)\n");
        let _ = write!(f, "# TYPE romm_category gauge\n");
        for (category, count) in &self.entries {
            // let _ = write!(f, "romm_category{{category=\\\"{}\\\"}} {}\n", category, count);
            let _ = write!(f, "romm_category{{category=\"{}\"}} {}\n", category, count);
        }
        Ok(())
    }
}

// Option 4: ROM file structure counts
impl Display for FileStructureCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = write!(f, "# HELP romm_file_structure Number of ROMs by storage format type\n");
        let _ = write!(f, "# TYPE romm_file_structure gauge\n");
        for (typ, count) in &self.entries {
            let _ = write!(f, "romm_file_structure{{type=\"{}\"}} {}\n", typ, count);
        }
        Ok(())
    }
}

// Manual counts: ROMs with vs without a manual
impl Display for ManualCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = write!(f, "# HELP romm_roms_with_manual Number of ROMs that have a manual\n");
        let _ = write!(f, "# TYPE romm_roms_with_manual gauge\n");
        for (flag, count) in &self.entries {
            let _ = write!(f, "romm_roms_with_manual{{has_manual=\"{}\"}} {}\n", flag, count);
        }
        Ok(())
    }
}

// Integrity state counts: ROMs by integrity state
impl Display for IntegrityCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = write!(f, "# HELP romm_rom_integrity Number of ROMs by integrity state (identified, unidentified, physical, missing)\n");
        let _ = write!(f, "# TYPE romm_rom_integrity gauge\n");
        for (state, count) in &self.entries {
            let _ = write!(f, "romm_rom_integrity{{state=\"{}\"}} {}\n", state, count);
        }
        Ok(())
    }
}

// Hash coverage: ROMs by available hash checksum type
impl Display for HashCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = write!(f, "# HELP romm_hash_coverage Number of ROMs by available hash checksum type\n");
        let _ = write!(f, "# TYPE romm_hash_coverage gauge\n");
        for (hash_type, count) in &self.entries {
            let _ = write!(f, "romm_hash_coverage{{hash_type=\"{}\"}} {}\n", hash_type, count);
        }
        Ok(())
    }
}


pub struct ApiClient {
    client: Client,
    base_url: String,
    auth_username: String,
    auth_password: String,
}

impl ApiClient {
    /// Initialize a new client with a base URL
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            auth_username: username.to_string(),
            auth_password: password.to_string(),
        }
    }

    /// Performs the login and returns an AuthResponse
    async fn login(&self, username: &str, password: &str) -> Result<AuthResponse, Box<dyn Error>> {
        let url = format!("{}/api/login", self.base_url);
        
        let payload = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        // Perform the POST request
        let response = self.client
            .post(url)
            .basic_auth(payload.username, Some(payload.password))
            .send()
            .await?;

        // Check if the status is 200 OK
        if response.status().is_success() {
            // NOTE: ROMM login gives empty body anyway for login
            let auth_data: AuthResponse = AuthResponse { token: String::from("") } ;
            Ok(auth_data)
        } else {
            let error_text = String::from("hello") ;
            Err(format!("Login failed with status {}: {}", response.status(), error_text).into())
        }
    }

    async fn stats(&self) -> Result<StatsResponse, Box<dyn Error>> {
        let url = format!("{}/api/stats", self.base_url);

        let response = self.client
            .get(url)
            .send()
            .await?;

        if response.status().is_success() {
            let auth_data: StatsResponse = response.json().await?;
            Ok(auth_data)
        } else {
            let error_text = String::from("hello");
            Err(format!("Stats failed with status {}: {}", response.status(), error_text).into())
        }
    }

    // Option 1 & 2: Fetch /api/platforms for per-platform ROM and firmware counts
    async fn platforms(&self) -> Result<PlatformsResponse, Box<dyn Error>> {
        let url = format!("{}/api/platforms", self.base_url);

        let response = self.client
            .get(url)
            .basic_auth(self.auth_username.to_string(), Some(self.auth_password.to_string()))
            .send()
            .await?;

        if response.status().is_success() {
            let data: Vec<PlatformEntry> = response.json().await?;
            Ok(PlatformsResponse::from_array(data))
        } else {
            let error_text = String::from("hello");
            Err(format!("Platforms failed with status {}: {}", response.status(), error_text).into())
        }
    }

    // Options 3, 4, 5, 6 & 7: Fetch /api/roms with pagination, aggregate category, file structure, manual, integrity, and hash counts
    async fn roms(&self) -> Result<(RomCategories, FileStructureCounts, ManualCounts, IntegrityCounts, HashCounts), Box<dyn Error>> {
        let mut category_entries: Vec<(String, i64)> = Vec::new();
        let mut fs_entries: Vec<(String, i64)> = Vec::new();
        let mut manual_entries: Vec<(String, i64)> = Vec::new();
        let mut integrity_entries: Vec<(String, i64)> = Vec::new();
        let mut hash_entries: Vec<(String, i64)> = Vec::new();

        let mut offset = 0;
        loop {
            let url = format!("{}/api/roms?offset={}&limit=100", self.base_url, offset);

            let response = self.client
                .get(url)
                .basic_auth(self.auth_username.to_string(), Some(self.auth_password.to_string()))
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = String::from("hello");
                return Err(format!("ROMs offset {} failed with status {}: {}", offset, response.status(), error_text).into());
            }

            let rom_data: RomPageData = response.json().await?;
            let roms = rom_data.items;

            if roms.is_empty() {
                break;
            }

            for rom in &roms {
                // Option 3: Parse category from name prefix (e.g., "~games~SomeGame" -> "games")
                let name = rom.name.clone();
                let mut category: String = String::from("unknown");
                let first_pos = name.find('~');
                if let Some(fp) = first_pos {
                    let rest = &name[fp + 1..];
                    let second_pos = rest.find('~');
                    if let Some(sp) = second_pos {
                        category = String::from(&rest[0..sp]);
                    }
                }
                // Update category count
                let mut found = false;
                let mut idx = category_entries.len();
                let mut i = 0;
                while i < category_entries.len() {
                    if category_entries[i].0 == category {
                        idx = i;
                        found = true;
                        break;
                    }
                    i += 1;
                }
                if found {
                    category_entries[idx].1 += 1;
                } else {
                    category_entries.push((String::from(category), 1));
                }

                // Option 4: Count by file structure type, derived from API fields
                let fs_type = if rom.has_simple_single_file {
                    "single"
                } else if rom.has_nested_single_file {
                    "nested_folder"
                } else if rom.has_multiple_files {
                    "multiple"
                } else if rom.is_easyrpg_game {
                    "easyrpg"
                } else {
                    "other"
                };
                // Update file structure count
                let mut found = false;
                let mut idx = fs_entries.len();
                let mut i = 0;
                while i < fs_entries.len() {
                    if fs_entries[i].0 == fs_type {
                        idx = i;
                        found = true;
                        break;
                    }
                    i += 1;
                }
                if found {
                    fs_entries[idx].1 += 1;
                } else {
                    fs_entries.push((String::from(fs_type), 1));
                }

                // Option 5: Count by has_manual flag
                let manual_flag = if rom.has_manual {
                    "true"
                } else {
                    "false"
                };
                let mut found = false;
                let mut idx = manual_entries.len();
                let mut i = 0;
                while i < manual_entries.len() {
                    if manual_entries[i].0 == manual_flag {
                        idx = i;
                        found = true;
                        break;
                    }
                    i += 1;
                }
                if found {
                    manual_entries[idx].1 += 1;
                } else {
                    manual_entries.push((String::from(manual_flag), 1));
                }

                // Option 6: Count by integrity state
                let state = if rom.is_unidentified {
                    "unidentified"
                } else if rom.is_identified {
                    "identified"
                } else if rom.missing_from_fs {
                    "missing"
                } else if rom.is_physical {
                    "physical"
                } else {
                    "unknown"
                };
                let mut found = false;
                let mut idx = integrity_entries.len();
                let mut i = 0;
                while i < integrity_entries.len() {
                    if integrity_entries[i].0 == state {
                        idx = i;
                        found = true;
                        break;
                    }
                    i += 1;
                }
                if found {
                    integrity_entries[idx].1 += 1;
                } else {
                    integrity_entries.push((String::from(state), 1));
                }

                // Option 7: Count by hash coverage (CRC, MD5, SHA1, RHash, none)
                let mut hash_types: Vec<String> = Vec::new();
                if !rom.crc_hash.is_empty() {
                    hash_types.push(String::from("CRC"));
                }
                if !rom.md5_hash.is_empty() {
                    hash_types.push(String::from("MD5"));
                }
                if !rom.sha1_hash.is_empty() {
                    hash_types.push(String::from("SHA1"));
                }
                if !rom.ra_hash.is_empty() {
                    hash_types.push(String::from("RHash"));
                }
                if hash_types.is_empty() {
                    hash_types.push(String::from("none"));
                }
                for ht in &hash_types {
                    let mut found = false;
                    let mut idx = hash_entries.len();
                    let mut i = 0;
                    while i < hash_entries.len() {
                        if hash_entries[i].0 == *ht {
                            idx = i;
                            found = true;
                            break;
                        }
                        i += 1;
                    }
                    if found {
                        hash_entries[idx].1 += 1;
                    } else {
                        hash_entries.push((String::from(ht), 1));
                    }
                }
            }

            // If we got fewer ROMs than the typical page size (e.g., 100), we've reached the last page
            if roms.len() < 100 {
                break;
            }

            offset += 100;
        }

        Ok((
            RomCategories { entries: category_entries },
            FileStructureCounts { entries: fs_entries },
            ManualCounts { entries: manual_entries },
            IntegrityCounts { entries: integrity_entries },
            HashCounts { entries: hash_entries },
        ))
    }

}

fn require_env(name: &str) -> Result<String, Box<dyn Error>> {
    match std::env::var_os(name) {
        Some(s) => {
            let s_str = s.to_str();
            match s_str {
                Some(str) => Ok(String::from(str)),
                None => Err(format!("Environment variable {} contains invalid UTF-8", name).into()),
            }
        },
        None => Err(format!("Missing required environment variable: {}", name).into()),
    }
}


#[tokio::main]
async fn main()  -> Result<(), Box<dyn Error>>{

    let username = require_env("ROMM_USERNAME")?;
    let password = require_env("ROMM_PASSWORD")?;
    let url = require_env("ROMM_URL")?;

    let api = ApiClient::new(&url, &username, &password);

    // Quick login check at startup to surface config errors early
    match api.login(&username, &password).await {
        Ok(auth) => {
            println!("Login successful! Token: {}", auth.token);
        },
        Err(e) => {
            eprintln!("Warning: Login failed: {}", e);
        },
    }

    // Initial fetch to populate metrics immediately
    let initial_metrics = fetch_all_metrics(&username, &password, &url).await;
    GLOBAL_METRICS.lock().unwrap().clone_from(&initial_metrics);

    // Background refresh loop: re-fetch metrics every N seconds (default 60)
    let interval: i32 = match std::env::var("ROMM_REFRESH_INTERVAL") {
        Ok(s) => s.parse().unwrap_or(60),
        Err(_) => 60,
    };
    println!("Background refresh interval: {}s", interval);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(interval as u64)).await;
            let metrics = fetch_all_metrics(&username, &password, &url).await;
            GLOBAL_METRICS.lock().unwrap().clone_from(&metrics);
        }
    });

    // Build our application with a single route
    let app = Router::new()
        .route("/", get(root))
        .route("/metrics", get(metrics_handler));

    // Define the address (localhost:3000)
    // TODO: add env variable to change port
    let addr = SocketAddr::from_str("127.0.0.1:3000").unwrap();
    println!("Server running at http://{}", addr);

    // Start the hyper/axum server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// Core fetch function: does all API calls and returns combined Prometheus metrics
async fn fetch_all_metrics(username: &str, password: &str, base_url: &str) -> String {
    let api = ApiClient::new(base_url, username, password);

    // Login (fire-and-forget for background fetch)
    let _ = api.login(username, password).await;

    // Fetch stats
    let stats = match api.stats().await {
        Ok(s) => s.clone(),
        Err(_) => StatsResponse::default(),
    };

    // Fetch platforms
    let platforms = match api.platforms().await {
        Ok(p) => p.clone(),
        Err(_) => PlatformsResponse::default(),
    };

    // Fetch ROMs (paginated)
    let (rom_cats, file_struct, manual_counts, integrity_counts, hash_counts) = match api.roms().await {
        Ok((c, fs, m, i, h)) => (c, fs, m, i, h),
        Err(_) => (RomCategories::default(), FileStructureCounts::default(), ManualCounts::default(), IntegrityCounts::default(), HashCounts::default()),
    };

    let stats_metrics: String = stats.to_string().to_string();
    let platforms_metrics: String = platforms.to_string().to_string();
    let rom_categories_metrics: String = rom_cats.to_string().to_string();
    let file_structure_metrics: String = file_struct.to_string().to_string();
    let manual_counts_metrics: String = manual_counts.to_string().to_string();
    let integrity_metrics: String = integrity_counts.to_string().to_string();
    let hash_metrics: String = hash_counts.to_string().to_string();
    stats_metrics + "\n" + &platforms_metrics + "\n" + &rom_categories_metrics + "\n" + &file_structure_metrics + "\n" + &manual_counts_metrics + "\n" + &integrity_metrics + "\n" + &hash_metrics
}

// Simple handler: returns the latest pre-fetched metrics from the background task
async fn metrics_handler() -> String {
    GLOBAL_METRICS.lock().unwrap().to_string()
}

async fn root() -> &'static str {
    "See ROMM Metrics on /metrics"
}
