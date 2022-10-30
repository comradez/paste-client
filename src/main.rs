use anyhow::{Context, Result};

use crate::config::Config;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use reqwest::{
    header::CONTENT_LENGTH,
    multipart::{Form, Part},
    Body,
};
use serde_derive::{Deserialize, Serialize};
use std::{
    io::{stdin, Read, Write},
    path::PathBuf,
    process::exit,
};
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{BytesCodec, FramedRead};

mod config;

const END_CHAR: char = if cfg!(target_os = "windows") {
    'Z'
} else {
    'D'
};

fn get_config(path: &PathBuf) -> Result<Config> {
    let mut f = std::fs::File::open(path)
        .context(format!("Failed to open file {}", path.as_path().display()))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    let config: Config = toml::from_str(&buf).context("Failed to parse toml")?;
    Ok(config)
}

fn save_history(token: &str, path: &PathBuf) -> Result<()> {
    let mut f = std::fs::File::create(path).context(format!(
        "Failed to create file {}",
        path.as_path().display()
    ))?;
    f.write_all(token.as_bytes()).context(format!(
        "Failed to write to file {}",
        path.as_path().display()
    ))?;
    Ok(())
}

fn read_history(path: &PathBuf) -> Result<String> {
    let mut f = std::fs::File::open(path)
        .context(format!("Failed to open file {}", path.as_path().display()))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf).context(format!(
        "Failed to read from file {}",
        path.as_path().display()
    ))?;
    Ok(buf)
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Get {
        token: String,
    },
    Send {
        message: Option<String>,
    },
    Delete {
        token: String,
    },
    Last,
    File {
        #[clap(subcommand)]
        command: V2Commands,
    },
}

#[derive(Subcommand)]
enum V2Commands {
    Get { filename: String },
    Send { path: PathBuf },
    Delete { filename: String },
}

#[derive(Serialize, Deserialize)]
struct ExchangeMessage {
    content: String,
    username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config_dir = home::home_dir()
        .expect("You must have a home directory to use this tool.")
        .join(".config")
        .join("paste-client");
    let config_path = config_dir.join("config.toml");
    let history_path = config_dir.join("history_token");

    let config = get_config(&config_path)?;
    let base_url = url::Url::parse(config.base_url.as_str())?;

    let client = if let Some(proxy) = config.proxy {
        reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(proxy)?)
            .build()?
    } else {
        reqwest::Client::new()
    };
    let cli = Cli::parse();
    match cli.command {
        Commands::Get { token } => {
            let url = base_url.join(token.as_str())?;
            let resp = client.get(url).send().await?;
            let message = resp.text().await?;
            println!("{}", message);
        }
        Commands::Send { message } => {
            let message = message.unwrap_or_else(|| {
                let mut content = String::new();
                println!("Input the message. Ctrl + {} to end.", END_CHAR);
                stdin().read_to_string(&mut content).unwrap();
                content
            });
            let resp = client.post(base_url.as_ref()).body(message).send().await?;
            let token = resp.text().await?;
            save_history(&token, &history_path).context("Failed to record history.")?;

            println!("\n{}", base_url.join(token.as_str())?);
        }
        Commands::Delete { token } => {
            let url = base_url.join(token.as_str())?;
            let resp = client.delete(url).send().await?;
            println!("{}", resp.text().await?);
        }
        Commands::Last => println!(
            "{}",
            base_url
                .join(
                    read_history(&history_path)
                        .context("No history recorded.")?
                        .as_str()
                )?
                .as_str(),
        ),
        Commands::File { command } => match command {
            V2Commands::Get { filename } => {
                let url = base_url.join("v2/")?.join(filename.as_str())?;
                let header_resp = client.head(url.as_ref()).send().await?;

                if let Some(content_length) = header_resp.headers().get(CONTENT_LENGTH) {
                    let content_length = content_length.to_str()?.parse::<u64>()?;
                    if content_length == 0 {
                        println!("File not exist.");
                        exit(0);
                    }
                    let mut downloaded = tokio::fs::File::create(&filename)
                        .await
                        .context("Failed to create file.")?;
                    let progress_bar = ProgressBar::new(content_length);
                    progress_bar
                        .set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
                        .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
                        .progress_chars("#>-"));
                    let mut resp = client.get(url.as_ref()).send().await?;
                    while let Some(bytes) = resp.chunk().await? {
                        progress_bar.inc(bytes.len() as u64);
                        downloaded.write_all(&bytes).await?;
                    }
                } else {
                    let mut downloaded = tokio::fs::File::create(&filename)
                        .await
                        .context("Failed to create file.")?;
                    println!("Downloading...");
                    let resp = client.get(url.as_ref()).send().await?;
                    downloaded.write_all(&resp.bytes().await?).await?;
                }

                println!("Downloaded file {}", &filename);
            }
            V2Commands::Send { path } => {
                let url = base_url.join("v2/")?;
                let filename = path
                    .file_name()
                    .context("This is not a file.")?
                    .to_string_lossy()
                    .into_owned();
                let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
                let mime = mime_guess::from_ext(ext).first_or_octet_stream();
                let file = tokio::fs::File::open(path.as_path()).await?;
                let part =
                    Part::stream(Body::wrap_stream(FramedRead::new(file, BytesCodec::new())))
                        .file_name(filename)
                        .mime_str(mime.essence_str())?;
                let form = Form::new().part("file", part);

                println!("Uploading...");
                let resp = client.post(url).multipart(form).send().await?;
                println!("{}", resp.text().await?);
            }
            V2Commands::Delete { filename } => {
                let url = base_url.join("v2/")?.join(filename.as_str())?;
                let resp = client.delete(url).send().await?;
                println!("{}", resp.text().await?);
            }
        },
    }
    Ok(())
}
