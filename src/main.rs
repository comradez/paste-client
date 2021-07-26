extern crate clap;
use clap::{Arg, App};
use std::io::{stdin, Read};

fn main() -> std::io::Result<()> {
    let matches = App::new("MyPaste")
        .version("0.1.0")
        .author("Chris Zhang <zcyjim@outlook.com>")
        .about("My command line pastebin client")
        .arg(Arg::with_name("destination")
            .short("d")
            .long("dest")
            .value_name("DEST")
            .help("Sets the destination of the pastebin")
            .takes_value(true))
        .arg(Arg::with_name("method")
            .short("m")
            .long("method")
            .value_name("METHOD")
            .possible_values(&["get", "g", "send", "s", "delete", "d"])
            .case_insensitive(true)
            .help("Determines whether to send or to get")
            .takes_value(true))
        .arg(Arg::with_name("content")
            .short("c")
            .long("content")
            .help("The content to send")
            .takes_value(true))
        .get_matches();
    let method = matches.value_of("method").unwrap_or("send");
    let mut content = String::new();
    let mut destination = String::new();
    if method == "send" || method == "s" {
        if let Some(line) = matches.value_of("content") {
            content = String::from(line);
        } else {
            println!("Input the message. Ctrl + D to end.");
            stdin().read_to_string(&mut content).unwrap();
        }
    } else if method == "delete" || method == "d" || method == "get" || method == "g" {
        if let Some(line) = matches.value_of("destination") {
            destination = String::from(line);
        } else {
            println!("Input the destination. Ctrl + D to end.");
            stdin().read_to_string(&mut destination).unwrap();
        }
    }
    let base_url = "SORRYBUTICANTTELLYOUTHIS";
    let url = format!("{}/{}", base_url, &destination);
    match method {
        "get" | "g" => { println!("{}", reqwest::blocking::get(url).unwrap().text().unwrap()); },
        "send" | "s" => {
            let client = reqwest::blocking::Client::new();
            let resp = client.post(base_url).body(content).send().unwrap();
            println!("\n{}", resp.text().unwrap());
        },
        "delete" | "d" => {
            let client = reqwest::blocking::Client::new();
            let resp = client.delete(url).body(content).send().unwrap();
            println!("{}", resp.text().unwrap());
        }
        _ => {}
    }
    Ok(())
}