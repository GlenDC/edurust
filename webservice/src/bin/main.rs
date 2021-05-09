use std::fs;
use std::thread;
use std::time::Duration;

use webservice::{HTTPServer, HTTPMethod};

fn main() {
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

    server.listen(7878).unwrap();

    println!("Shutting down.");
}
