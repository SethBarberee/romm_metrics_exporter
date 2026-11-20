use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;

use reqwest::Client;

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
#[serde(rename_all = "UPPERCASE")]
struct StatsResponse {
    platforms: i64,
    roms: i64,
    saves: i64,
    states: i64,
    screenshots: i64,
    total_filesize_bytes: i64,
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

impl Default for PlatformsResponse {
    fn default() -> Self {
        Self { platforms: Vec::<PlatformEntry>::default() }
    }
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
            // .json(&payload) // Automatically sets Content-Type to application/json
            .basic_auth(payload.username, Some(payload.password))
            .send()
            .await?;

        // Check if the status is 200 OK
        if response.status().is_success() {
            // NOTE: ROMM login gives empty body anyway for login
            let auth_data: AuthResponse = AuthResponse { token: String::from("") } ;
            Ok(auth_data)
        } else {
            // Handle errors (e.g., 401 Unauthorized or 500 Server Error)
            // let error_text = response.text().await? ;
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
            Ok(PlatformsResponse { platforms: data })
        } else {
            let error_text = String::from("hello");
            Err(format!("Platforms failed with status {}: {}", response.status(), error_text).into())
        }
    }

}

#[tokio::main]
async fn main()  -> Result<(), Box<dyn Error>>{

    let username = std::env::var_os("ROMM_USERNAME").unwrap();
    let password = std::env::var_os("ROMM_PASSWORD").unwrap();
    let url = std::env::var_os("ROMM_URL").unwrap();

    let api = ApiClient::new(url.to_str().unwrap(), username.to_str().unwrap(), password.to_str().unwrap());

    match api.login(username.to_str().unwrap(), password.to_str().unwrap()).await {
        Ok(auth) => {
            println!("Login successful! Token: {}", auth.token);
        },
        Err(e) => {
            eprintln!("Error during login: {}", e);
        },
    }

    let mut stats= StatsResponse::default();
    let mut platforms: PlatformsResponse = PlatformsResponse { platforms: Vec::<PlatformEntry>::default() };

    match api.stats().await {
        Ok(auth) => {
            println!("Stats successful!");
            stats = auth.clone();
        },
        Err(e) => {
            eprintln!("Error during stats : {}", e);
        },
    }

    match api.platforms().await {
        Ok(auth) => {
            println!("Platforms successful!");
            platforms = auth.clone();
        },
        Err(e) => {
            eprintln!("Error during platforms: {}", e);
        },
    }

    // Debug: print raw stats and platforms data for inspection
    eprintln!("Stats loaded: platforms={}, roms={}", stats.platforms, stats.roms);
    eprintln!("Platforms loaded: {} entries", platforms.platforms.len());

    let stats_metrics: String = stats.to_string().to_string();
    let platforms_metrics: String = platforms.to_string().to_string();
    let combined_metrics = stats_metrics + "\n" + &platforms_metrics;

    // Build our application with a single route
    let app = Router::new()
        .route("/", get(root))
        // .route("/metrics", get(metrics_handler));
        .route("/metrics", get(combined_metrics));

    // Define the address (localhost:3000)
    // TODO: add env variable to change port
    let addr = SocketAddr::from_str("127.0.0.1:3000").unwrap();
    println!("Server running at http://{}", addr);

    // Start the hyper/axum server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn root() -> &'static str {
    "See ROMM Metrics on /metrics"
}

/// Handler for the /metrics endpoint
async fn metrics_handler() -> &'static str {
    // You can replace this placeholder text with your actual metrics logic later
    "# HELP application_memory_usage Total memory used by the application\n\
     # TYPE application_memory_usage gauge\n\
     application_memory_usage 12345678\n\
     \n\
     # HELP request_count_total Total number of requests received\n\
     # TYPE request_count_total counter\n\
     request_count_total 500"
}
