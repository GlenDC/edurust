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

// Executor used to handle a connection.
pub type HandleExecutor = Box<dyn FnMut(HandleFn) + 'static>;

// Function given to a handle executor to handle a connection.
pub type HandleFn = Box<dyn FnOnce() + Send + 'static>;

/// Minimal HTTP Server, that can be used
/// to handle the most simple HTTP calls.
pub struct HTTPServer {
    handles: HashMap<String, HTTPHandle>,
    shutdown: Option<mpsc::Receiver<()>>,
    executor: Option<HandleExecutor>,
}

impl HTTPServer {
    /// Create a new HTTP Server.
    pub fn new() -> HTTPServer {
        HTTPServer {
            handles: HashMap::new(),
            shutdown: None,
            executor: None,
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

    /// Add a receiver that is to be send an empty value,
    /// in order to trigger a graceful shutdown.
    pub fn set_shutdown(&mut self, r: mpsc::Receiver<()>) {
        self.shutdown = Some(r);
    }

    /// Set a custom (pool) executor that will be called to
    /// handle a connection. Allowing you to implement a custom
    /// thread pool instead of the default [ThreadPool][self::thread::ThreadPool],
    /// or to even do so in a concurrent fashion.
    pub fn set_handle_executor(&mut self, f: HandleExecutor) {
        self.executor = Some(f);
    }

    /// Listen on the given local TCP port for incoming requests,
    /// consuming this [HTTPServer](self::HTTPServer) and serving content
    /// using the added [handlers](self::HTTPHandle).
    pub fn listen(mut self, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        listener.set_nonblocking(true)?;

        log::info!("HTTP Server listening at: {}", listener.local_addr()?);

        let mut execute = match self.executor {
            Some(e) => e,
            None => {
                let pool = ThreadPool::new(4).unwrap();
                Box::new(move |f| {
                    pool.execute(f);
                })
            }
        };

        let handles = Arc::new(self.handles);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handles = Arc::clone(&handles);
                    execute(Box::new(move || {
                        match handle_connection(handles, stream) {
                            Err(e) =>  log::error!("failed to handle connection: {}", e),
                            Ok(_) => (),
                        };
                    }));
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
    for _ in 0..16 {  // retry a max amount of times
        match stream.read(&mut buffer) {
            Ok(_) => break,
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    _ => return Err(e),
                }
            }
        }
    }
    if buffer[0] == 0 {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
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
            log::debug!("TCP Request matched: {:?}", String::from_utf8_lossy(&buffer).trim_end_matches('\u{0}'));
            return handle(Box::new(cb));
        }
    }

    log::debug!("404 response for TCP Request: {:?}", String::from_utf8_lossy(&buffer).trim_end_matches('\u{0}'));
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
