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
        let _ = write!(f, "romm_platforms {} \n", self.platforms);

        let _ = write!(f, "# HELP romm_roms A metric for number of ROMs in ROMM\n");
        let _ = write!(f, "# TYPE romm_roms gauge\n");
        let _ = write!(f, "romm_roms {} \n" , self.roms);

        let _ = write!(f, "# HELP romm_saves A metric for number of saves in ROMM\n");
        let _ = write!(f, "# TYPE romm_saves gauge\n");
        let _ = write!(f, "romm_saves {} \n" , self.saves);

        let _ = write!(f, "# HELP romm_states A metric for number of save states in ROMM\n");
        let _ = write!(f, "# TYPE romm_states gauge\n");
        let _ = write!(f, "romm_states {} \n" , self.states);

        let _ = write!(f, "# HELP romm_screenshots A metric for number of screenshots in ROMM\n");
        let _ = write!(f, "# TYPE romm_screenshots gauge\n");
        let _ = write!(f, "romm_screenshots {} \n", self.screenshots);

        let _ = write!(f, "# HELP romm_total_filesize_types A metric for the total filesize of ROMM\n");
        let _ = write!(f, "# TYPE romm_total_filesize_types gauge\n");
        write!(f, "romm_total_filesize_types {} \n", self.total_filesize_bytes)
    }
}


pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    /// Initialize a new client with a base URL
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
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
        
        // Perform the POST request
        let response = self.client
            .get(url)
            // .json(&payload) // Automatically sets Content-Type to application/json
            .send()
            .await?;

        // Check if the status is 200 OK
        if response.status().is_success() {
            let auth_data: StatsResponse = response.json().await?;
            Ok(auth_data)
        } else {
            // Handle errors (e.g., 401 Unauthorized or 500 Server Error)
            // let error_text = response.text().await? ;
            let error_text = String::from("hello") ;
            Err(format!("Stats failed with status {}: {}", response.status(), error_text).into())
        }
    }

}

#[tokio::main]
async fn main()  -> Result<(), Box<dyn Error>>{

    let username = std::env::var_os("ROMM_USERNAME").unwrap();
    let password = std::env::var_os("ROMM_PASSWORD").unwrap();
    let url = std::env::var_os("ROMM_URL").unwrap();

    let api = ApiClient::new(url.to_str().unwrap());

    match api.login(username.to_str().unwrap(), password.to_str().unwrap()).await {
        Ok(auth) => {
            println!("Login successful! Token: {}", auth.token);
        },
        Err(e) => {
            eprintln!("Error during login: {}", e);
        },
    }

    let mut stats= StatsResponse::default();

    match api.stats().await {
        Ok(auth) => {
            println!("Stats successful!");
            stats = auth.clone();
        },
        Err(e) => {
            eprintln!("Error during stats : {}", e);
        },
    }


    // Build our application with a single route
    let app = Router::new()
        .route("/", get(root))
        // .route("/metrics", get(metrics_handler));
        .route("/metrics", get(stats.to_string()));

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
