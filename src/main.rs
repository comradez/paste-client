extern crate clap;
use clap::{Parser, Subcommand};
use config::Config;
use serde_derive::{Serialize, Deserialize};
use std::{io::{stdin, Read, Write}, path::{PathBuf}};

mod config;

fn get_url(path: &PathBuf) -> std::io::Result<Config> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    let config: Config = toml::from_str(&buf).expect("Toml parse error.");
    Ok(config)
}

fn set_username(username: &str, path: &PathBuf, base_url: &str) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    let config = Config {
        base_url: base_url.into(),
        username: Some(username.into())
    };
    f.write_all(toml::to_string(&config).unwrap().as_bytes())?;
    Ok(())
}

fn save_history(token: &str, path: &PathBuf) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(token.as_bytes())?;
    Ok(())
}

fn read_history(path: &PathBuf) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok(buf)
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    Get { token: String },
    Send { message: Option<String> },
    Delete { token: String },
    Username { name: String },
    Last
}

#[derive(Serialize, Deserialize)]
struct ExchangeMessage {
    content: String,
    username: String,
}

fn main() -> std::io::Result<()> {
    let mut home_dir = home::home_dir().unwrap();
    home_dir.push(".config");
    home_dir.push("paste-client");
    let config_path = {
        let mut config_path = home_dir.clone();
        config_path.push("config.toml");
        config_path
    };
    let history_path = {
        let mut history_path = home_dir;
        history_path.push("history_token");
        history_path
    };
    let config = get_url(&config_path).unwrap();
    let (base_url, username) = (config.base_url, config.username.unwrap_or("Anonymous".into()));
    let end_char = if cfg!(target_os = "windows") { 'Z' } else { 'D' };

    let cli = Cli::parse();
    match cli.command {
        Commands::Get { token } => {
            let url = format!("{}/{}", base_url, token);
            let message: ExchangeMessage = serde_json::from_str(&reqwest::blocking::get(url).unwrap().text().unwrap()).unwrap();
            println!("User {} pasted:", &message.username);
            println!("{}", &message.content);
        },
        Commands::Send { message } => {
            let content = message.unwrap_or_else(|| {
                let mut content = String::new();
                println!("Input the message. Ctrl + {} to end.", end_char);
                stdin().read_to_string(&mut content).unwrap();
                content
            });
            let message = serde_json::to_string(&ExchangeMessage {
                content,
                username: username.into()
            }).unwrap();
            let client = reqwest::blocking::Client::new();
            let resp = client.post(&base_url).body(message).send().unwrap();
            let token = resp.text().unwrap();
            save_history(&token, &history_path).expect("Failed to record history");
            println!("\n{}/{}", &base_url, &token);
        },
        Commands::Delete { token } => {
            let url = format!("{}/{}", base_url, token);
            let client = reqwest::blocking::Client::new();
            let resp = client.delete(url).send().unwrap();
            println!("{}", resp.text().unwrap());
        },
        Commands::Username { name } => set_username(&name, &config_path, &base_url)?,
        Commands::Last => println!(
            "{}/{}",
            base_url,
            read_history(&history_path).unwrap_or("No history recorded!".to_string())
        ),
    }
    Ok(())
}
