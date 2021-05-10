//! A very minimal HTTP Server allowing you to server
//! header-less content over GET/POST methods,
//! without the ability to inspect received headers or use of query parameters.
//! 
//! Really a useless HTTP server, and served only to allow the author
//! to get some experience in writing a small multi-threaded library with stored closures.
//! 
//! # Example
//! 
//! ```
//! use webservice::{HTTPServer, HTTPMethod};
//! 
//! let mut server = HTTPServer::new();
//!
//! server.add_handle(HTTPMethod::GET, "/", Box::new(|mut cb| {
//!     cb(200, Some(r#"<!DOCTYPE html>
//! <html lang="en">
//! <head>
//!   <meta charset="utf-8">
//!   <title>Hello!</title>
//! </head>
//! <body>
//!   <h1>Hi!</h1>
//!   <p>Have a nice day.</p>
//! </body>
//! </html>
//! "#))
//! }));
//!
//! // Start to listen:
//! // server.listen(0).unwrap();
//! ```

use std::io;
use std::io::prelude::*;
use std::time::Duration;
use std::fmt;
use std::sync::Arc;
use std::sync::mpsc;
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};

use log;

pub mod thread;

use self::thread::ThreadPool;

/// Typed definitions of the HTTP methods supported by this server.
pub enum HTTPMethod {
    GET,
    POST,
}

/// Unrestricted HTTP Status codes, as the author is too lazy
/// to define them here.
pub type HTTPStatus = u32;

impl fmt::Display for HTTPMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            HTTPMethod::GET => "GET",
            HTTPMethod::POST => "POST",
        })
    }
}

/// Callback given to any [HTTPHandle](self::HTTPHandle)
/// giving it the ability to write a response back to the user.
pub type HTTPHandleCallback = Box<dyn FnMut(HTTPStatus, Option<&str>) -> io::Result<()> + Sync + Send + 'static>;

/// Definition of an HTTP Handle that can be added to an [HTTPServer](self::HTTPServer)
/// in order to serve content for a static path for a specific method.
pub type HTTPHandle = Box<dyn Fn(HTTPHandleCallback) -> io::Result<()> + Sync + Send + 'static>;

/// Minimal HTTP Server, that can be used
/// to handle the most simple HTTP calls.
pub struct HTTPServer {
    handles: HashMap<String, HTTPHandle>,
    shutdown: Option<mpsc::Receiver<()>>,
}

impl HTTPServer {
    /// Create a new HTTP Server.
    pub fn new() -> HTTPServer {
        HTTPServer {
            handles: HashMap::new(),
            shutdown: None,
        }
    }

    /// Add an HTTP Handle for a specific method and path,
    /// such that when the user makes a request to it,
    /// the given handle can provide the response status code
    /// and optionally also content.
    /// 
    /// Note:
    /// - No headers can be given;
    /// - Path won't be matched if query parameters were given by the user;
    /// - Existing handle with same path and method will be overwritten in silence.
    pub fn add_handle(&mut self, method: HTTPMethod, path: &str, handle: HTTPHandle) {
        let pattern = String::from(format!("{} {} HTTP/1.1\r\n", method, path));
        self.handles.insert(pattern, handle);
    }

    pub fn set_shutdown(&mut self, r: mpsc::Receiver<()>) {
        self.shutdown = Some(r);
    }

    /// Listen on the given local TCP port for incoming requests,
    /// consuming this [HTTPServer](self::HTTPServer) and serving content
    /// using the added [handlers](self::HTTPHandle).
    pub fn listen(mut self, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        listener.set_nonblocking(true)?;

        log::info!("HTTP Server listening at: {}", listener.local_addr()?);

        let pool = ThreadPool::new(4).unwrap();
        let handles = Arc::new(self.handles);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handles = Arc::clone(&handles);
                    pool.execute(move || {
                        match handle_connection(handles, stream) {
                            Err(e) =>  log::error!("failed to handle connection: {}", e),
                            Ok(_) => (),
                        };
                    });
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if let Some(ref shutdown) = self.shutdown {
                        match shutdown.try_recv() {
                            Err(e) => {
                                if e == mpsc::TryRecvError::Empty {
                                    continue;
                                }
                                log::error!("graceful shutdown channel was set, but has an unexpected error: {}", e);
                                self.shutdown = None;
                            }
                            Ok(_) => {
                                log::info!("Graceful shutdown signal received, stopping server now...");
                                break;
                            }
                        }
                    };
                }
                Err(e) => {
                    eprintln!("failed to handle connection: encountered IO error: {}", e);
                }
            };
        }

        log::debug!("HTTP Server stopped listening!");
        Ok(())
    }
}

fn handle_connection(handles: Arc<HashMap<String, HTTPHandle>>, mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(_) => break,
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    _ => return Err(e),
                }
            }
        }
    }

    let mut cb = move |status, opt_content: Option<&str>| -> io::Result<()> {
        let response = match opt_content {
            Some(content) => format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n{}",
                status, content.len(), content,
            ),
            None => format!("HTTP/1.1 {}\n\r\n", status),
        };

        stream.write(response.as_bytes())?;
        stream.flush()
    };

    for (pattern, handle) in handles.iter() {
        if buffer.starts_with(pattern.as_bytes()) {
            return handle(Box::new(cb));
        }
    }

    cb(404, Some(HTTP_CONTENT_404))
}

const HTTP_CONTENT_404: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>Hello!</title>
  </head>
  <body>
    <h1>Oops!</h1>
    <p>Sorry, I don't know what you're asking for.</p>
  </body>
</html>
"#;
