use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use clap::{AppSettings, Clap};
use env_logger::Builder;

use webservice::{HTTPMethod, HTTPServer, HandleFn};

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
    /// Define how the TCP connections are handled (default, crate, blocked)
    #[clap(long, default_value = "default")]
    handle: HandleMethod,
}

// HandleMethod allows you to define how TCP connections are handled.
enum HandleMethod {
    // Use the shipped thread pool to handle connections concurrently.
    Default,
    // Use the used thread pool crate to handle connections concurrently.
    Crate,
    // Handle each http connection in a blocked manner.
    Blocked,
}

impl std::str::FromStr for HandleMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<HandleMethod, String> {
        match s.to_lowercase().trim() {
            "" | "def" | "default" => Ok(HandleMethod::Default),
            "crate" | "lib" => Ok(HandleMethod::Crate),
            "block" | "blocked" => Ok(HandleMethod::Blocked),
            _ => Err(format!("string cannot be parsed to HandleMethod: {}", s)),
        }
    }
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
    let mut server: HTTPServer = Default::default();

    // set handle executor
    match opts.handle {
        HandleMethod::Crate => {
            let pool = threadpool::ThreadPool::new(4);
            let execute = move |f| {
                pool.execute(f);
            };
            server.set_handle_executor(Box::new(execute));
        }
        HandleMethod::Blocked => {
            let execute = |f: HandleFn| {
                f();
            };
            server.set_handle_executor(Box::new(execute));
        }
        // nothing to do, as this one will be used by default
        HandleMethod::Default => (),
    }

    // to handle graceful shutdown handling
    let (tx, rx) = mpsc::channel();
    server.set_shutdown(rx);

    // add all handlers

    server.add_handle(
        HTTPMethod::Get,
        "/",
        Box::new(|mut cb| {
            let contents = fs::read_to_string("hello.html")?;
            cb(200, Some(&contents))
        }),
    );
    server.add_handle(
        HTTPMethod::Get,
        "/sleep",
        Box::new(|mut cb| {
            thread::sleep(Duration::from_secs(5));
            let contents = fs::read_to_string("hello.html")?;
            cb(200, Some(&contents))
        }),
    );
    server.add_handle(
        HTTPMethod::Get,
        "/forbidden",
        Box::new(|mut cb| cb(403, None)),
    );

    // add signal handling
    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    // start the server until we stop
    server.listen(opts.port).unwrap();
    log::info!("Shutting down.");
}
