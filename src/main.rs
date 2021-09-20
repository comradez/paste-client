extern crate clap;
extern crate dotenv;

use clap::{App, Arg};
use config::Config;
use dotenv::dotenv;

use std::env;
use std::io::{stdin, Read, Write};

mod config;

fn get_from_config(path: &str) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    let config: Config = toml::from_str(&buf).expect("Toml parse error.");
    Ok(config.base_url)
}

fn save_history(token: &str, path: &str) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(token.as_bytes())?;
    Ok(())
}

fn read_history(path: &str) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok(buf)
}

fn main() -> std::io::Result<()> {
    let end_char = if cfg!(target_os = "windows") {
        'Z'
    } else {
        'D'
    };

    let user_name = std::env::var("HOME").unwrap();
    let config_path = format!("/{}/.config/paste-client/config.toml", user_name);
    let hist_path = format!("/{}/.config/paste-client/history_token", user_name);

    dotenv().ok();
    let matches = App::new("MyPaste")
        .version("0.1.0")
        .author("Chris Zhang <zcyjim@outlook.com>")
        .about("My command line pastebin client")
        .arg(
            Arg::with_name("destination")
                .short("d")
                .long("dest")
                .value_name("DEST")
                .help("Sets the destination of the pastebin")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("method")
                .short("m")
                .long("method")
                .value_name("METHOD")
                .possible_values(&["g", "s", "d", "l"])
                .case_insensitive(true)
                .help("Determines whether to send, get or delete. l outputs the last token.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("content")
                .short("c")
                .long("content")
                .help("The content to send")
                .takes_value(true),
        )
        .get_matches();
    let method = matches.value_of("method").unwrap_or("s");
    let mut content = String::new();
    let mut destination = String::new();
    if method == "s" {
        if let Some(line) = matches.value_of("content") {
            content = String::from(line);
        } else {
            println!("Input the message. Ctrl + {} to end.", end_char);
            stdin().read_to_string(&mut content).unwrap();
        }
    } else if method == "d" || method == "g" {
        if let Some(line) = matches.value_of("destination") {
            destination = String::from(line);
        } else {
            println!("Input the destination. Ctrl + {} to end.", end_char);
            stdin().read_to_string(&mut destination).unwrap();
        }
    }
    let base_url = get_from_config(&config_path).unwrap_or_else(|_| env::var("BASE_URL").unwrap());
    let url = format!("{}/{}", base_url, &destination);
    match method {
        "g" => {
            println!("{}", reqwest::blocking::get(url).unwrap().text().unwrap());
        }
        "s" => {
            let client = reqwest::blocking::Client::new();
            let resp = client.post(base_url).body(content).send().unwrap();
            let token = resp.text().unwrap();
            save_history(&token, &hist_path).expect("Failed to record history");
            println!("\n{}", &token);
        }
        "d" => {
            let client = reqwest::blocking::Client::new();
            let resp = client.delete(url).body(content).send().unwrap();
            println!("{}", resp.text().unwrap());
        }
        "l" => println!(
            "{}",
            read_history(&hist_path).unwrap_or_else(|_| "No history recorded!".to_string())
        ),
        _ => {}
    }
    Ok(())
}
