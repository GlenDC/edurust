use std::fs;
use std::thread;
use std::time::Duration;
use std::sync::mpsc;

use env_logger::Builder;
use log;
use clap::{AppSettings, Clap};
use ctrlc;

use webservice::{HTTPServer, HTTPMethod};

/// A minimal HTTP server, responding to almost nothing.
#[derive(Clap)]
#[clap(version = "0.1", author = "Glen DC <contact@glendc.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// port to listen to for incoming TCP traffic
    #[clap(short, long, default_value = "7878")]
    port: u16,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

fn main() {
    let opts: Opts = Opts::parse();

    // set log level
    let log_filter = match opts.verbose {
        0 => log::LevelFilter::Info,
        _ => log::LevelFilter::Debug,
    };
    Builder::new().filter_level(log_filter).init();

    // create the HTTP server
    let mut server = HTTPServer::new();

    // to handle graceful shutdown handling
    let (tx, rx) = mpsc::channel();
    server.set_shutdown(rx);

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

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    }).expect("Error setting Ctrl-C handler");

    server.listen(opts.port).unwrap();

    log::info!("Shutting down.");
}
