extern crate clap;
use clap::{Arg, App};
use std::io::{stdin, Read};

fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
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
            .possible_values(&["get", "send"])
            .case_insensitive(true)
            .help("Determines whether to send or to get")
            .takes_value(true))
        .arg(Arg::with_name("content")
            .short("c")
            .long("content")
            .help("The content to send")
            .takes_value(true))
        .get_matches();
    let dest = matches.value_of("destination").unwrap_or("1");
    let method = matches.value_of("method").unwrap_or("get");
    let mut content = String::new();
    if method == "send" {
        if let Some(line) = matches.value_of("content") {
            content = String::from(line);
        } else {
            println!("Input the message. Ctrl + D to end.");
            stdin().read_to_string(&mut content).unwrap();
        }
    }
    let url = format!("{}/{}", std::env::var("BASE_URL").unwrap(), &dest);
    match method {
        "get" => { println!("{}", reqwest::blocking::get(url).unwrap().text().unwrap()); },
        "send" => {
            let client = reqwest::blocking::Client::new();
            client.post(url).body(content).send().unwrap();
        },
        _ => {}
    }
    Ok(())
}