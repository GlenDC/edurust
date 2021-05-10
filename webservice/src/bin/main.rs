use std::fs;
use std::thread;
use std::time::Duration;

use clap::{AppSettings, Clap};

use webservice::{HTTPServer, HTTPMethod};

/// A minimal HTTP server, responding to almost nothing.
#[derive(Clap)]
#[clap(version = "0.1", author = "Glen DC <contact@glendc.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// port to listen to for incoming TCP traffic
    #[clap(short, long, default_value = "7878")]
    port: u16,
}

fn main() {
    let opts: Opts = Opts::parse();

    let mut server = HTTPServer::new();

    server.add_handle(HTTPMethod::GET, "/", Box::new(|mut cb| {
        let contents = fs::read_to_string("hello.html")?;
        cb(200, Some(&contents))
    }));

    server.add_handle(HTTPMethod::GET, "/sleep", Box::new(|mut cb| {
        thread::sleep(Duration::from_secs(5));
        let contents = fs::read_to_string("hello.html")?;
        cb(200, Some(&contents))
    }));

    server.add_handle(HTTPMethod::GET, "/forbidden", Box::new(|mut cb| {
        cb(403, None)
    }));

    server.listen(opts.port).unwrap();

    println!("Shutting down.");
}
