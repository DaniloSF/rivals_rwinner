use std::{env, path::Path};

use config::Config;
use ini::Ini;
use lazy_static::lazy_static;
use std::sync::OnceLock;

// Initialize for the first time the settings from the config file when needed
lazy_static! {
    static ref SETTINGS: Config = {
        let config_path = Path::parent(env::current_exe().unwrap().as_path())
            .unwrap()
            .join("config.ini");
        let settings = get_settings(&config_path)
            .unwrap_or_else(|_| create_default_config(config_path.to_str().unwrap()).unwrap());
        settings
    };
}

static DATA_ADDRESS: OnceLock<String> = OnceLock::new();
static DEBUG_ADDRESS: OnceLock<String> = OnceLock::new();
static SEND_ADDRESS: OnceLock<String> = OnceLock::new();

// Get the settings from the config file
fn get_settings(config_path: &Path) -> color_eyre::Result<Config> {
    // check if the config file exists in the same directory as the executable

    if !config_path.exists() {
        // if it doesn't exist, create a new config file with rust-ini
        return Err(color_eyre::Report::msg(format!(
            "Config file not found at {}",
            config_path.to_str().unwrap()
        )));
    }

    let settings = Config::builder()
        .add_source(config::File::with_name(config_path.to_str().unwrap()))
        .build()?;
    Ok(settings)
}

fn create_default_config(config_path: &str) -> color_eyre::Result<Config> {
    let mut conf_ini = Ini::new();

    conf_ini
        .with_section(Some("internal_data"))
        .set("tcp_ip", "127.0.0.1")
        .set("tcp_port", "4555");
    conf_ini
        .with_section(Some("internal_debug"))
        .set("tcp_ip", "127.0.0.1")
        .set("tcp_port", "4556");
    conf_ini
        .with_section(Some("conn"))
        .set("tcp_ip", "127.0.0.1")
        .set("tcp_port", "5005");
    conf_ini.write_to_file(config_path)?;
    let conf = Config::builder()
        .add_source(config::File::with_name(config_path))
        .build()?;
    Ok(conf)
}

// Get the debug address from the config file
pub fn get_debug_address() -> String {
    let addr = DEBUG_ADDRESS.get_or_init(|| {
        let debug_tcp_ip = SETTINGS
            .get_string("internal_debug.tcp_ip")
            .unwrap_or("127.0.0.1".to_string());
        let debug_tcp_port = SETTINGS
            .get_string("internal_debug.tcp_port")
            .unwrap_or("4556".to_string());
        debug_tcp_ip + ":" + &debug_tcp_port
    });
    addr.to_string()
}

// Get the data address from the config file
pub fn get_data_address() -> String {
    let addr = DATA_ADDRESS.get_or_init(|| {
        let data_tcp_ip = SETTINGS
            .get_string("internal_data.tcp_ip")
            .unwrap_or("127.0.0.1".to_string());
        let data_tcp_port = SETTINGS
            .get_string("internal_data.tcp_port")
            .unwrap_or("4555".to_string());
        data_tcp_ip + ":" + &data_tcp_port
    });
    addr.to_string()
}

// Get the send address from the config file
pub fn get_send_address() -> String {
    let addr = SEND_ADDRESS.get_or_init(|| {
        let send_tcp_ip = SETTINGS
            .get_string("conn.tcp_ip")
            .unwrap_or("127.0.0.1".to_string());
        let send_tcp_port = SETTINGS
            .get_string("conn.tcp_port")
            .unwrap_or("5005".to_string());
        send_tcp_ip + ":" + &send_tcp_port
    });
    addr.to_string()
}
