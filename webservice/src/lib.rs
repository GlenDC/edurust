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
//! use webservice::{HTTPServer, HTTPMethod, HTTPResponse};
//! 
//! let mut server: HTTPServer = Default::default();
//! 
//! server.add_handle(HTTPMethod::Get, "/", Box::new(|| {
//!     Ok(HTTPResponse::new(200).with_content(r#"<!DOCTYPE html>
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

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

pub mod thread;

use self::thread::ThreadPool;

/// Typed definitions of the HTTP methods supported by this server.
pub enum HTTPMethod {
    Get,
    Post,
}

/// Unrestricted HTTP Status codes, as the author is too lazy
/// to define them here.
pub type HTTPStatus = u32;

impl fmt::Display for HTTPMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            HTTPMethod::Get => "GET",
            HTTPMethod::Post => "POST",
        })
    }
}

/// Response returned by an [HTTPHandle](self::HTTPHandle),
/// defining the status and optionally also content.
/// 
/// Only UTF-8 content is supported for simplicity sake.
/// For the same reason headers aren't supported either.
pub struct HTTPResponse {
    status: HTTPStatus,
    content: Option<String>,
}

impl HTTPResponse {
    /// Create a new [HTTPResponse](self::HTTPResponse) for
    /// a given [HTTPStatus](self::HTTPStatus),
    /// if content is desired as well it will have to set
    /// using the provided builder [with_content](self::HTTPResponse::with_content) method.
    pub fn new(status: HTTPStatus) -> HTTPResponse {
        HTTPResponse {
            status,
            content: None,
        }
    }

    /// Consume this [HTTPResponse](self::HTTPResponse) and return
    /// a new response with (UTF-8) content added to it.
    pub fn with_content(self, content: &str) -> HTTPResponse {
        HTTPResponse {
            content: Some(String::from(content)),
            ..self
        }
    }
}

impl fmt::Display for HTTPResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content = match &self.content {
            Some(content) => format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n{}",
                self.status,
                content.len(),
                content,
            ),
            None => format!("HTTP/1.1 {}\r\n\r\n", self.status),
        };
        f.write_str(&content)
    }
}

/// Definition of an HTTP Handle that can be added to an [HTTPServer](self::HTTPServer)
/// in order to serve content for a static path for a specific method.
pub type HTTPHandle = Box<dyn Fn() -> io::Result<HTTPResponse> + Sync + Send>;

// Executor used to handle a connection.
pub type HandleExecutor = Box<dyn FnMut(HandleFn)>;

// Function given to a handle executor to handle a connection.
pub type HandleFn = Box<dyn FnOnce() + Send>;

/// Minimal HTTP Server, that can be used
/// to handle the most simple HTTP calls.
pub struct HTTPServer {
    handles: HashMap<String, HTTPHandle>,
    shutdown: Option<mpsc::Receiver<()>>,
    executor: Option<HandleExecutor>,
}

impl Default for HTTPServer {
    fn default() -> Self {
        Self::new()
    }
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
        let pattern = create_pattern(method, path);
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
                        if let Err(e) = handle_connection(handles, stream) {
                            log::error!("failed to handle connection: {}", e);
                        }
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
                                log::info!(
                                    "Graceful shutdown signal received, stopping server now..."
                                );
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

fn create_pattern(method: HTTPMethod, path: &str) -> String {
    if path == "" {
        return create_pattern(method, "/");
    }
    format!("{} {} HTTP/1.1\r\n", method, path)
}

fn handle_connection(
    handles: Arc<HashMap<String, HTTPHandle>>,
    mut stream: impl Read + Write,
) -> io::Result<()> {
    let mut buffer = [0; 1024];
    for _ in 0..16 {
        // retry a max amount of times
        match stream.read(&mut buffer) {
            Ok(_) => break,
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                _ => return Err(e),
            },
        }
    }
    if buffer[0] == 0 {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }

    let mut response = None;

    for (pattern, handle) in handles.iter() {
        if buffer.starts_with(pattern.as_bytes()) {
            log::debug!(
                "TCP Request matched: {:?}",
                String::from_utf8_lossy(&buffer).trim_end_matches('\u{0}')
            );
            response = Some(handle()?)
        }
    }

    log::debug!(
        "404 response for TCP Request: {:?}",
        String::from_utf8_lossy(&buffer).trim_end_matches('\u{0}')
    );

    let content = format!("{}", match response {
        Some(resp) => resp,
        None => HTTPResponse::new(404).with_content(HTTP_CONTENT_404),
    });
    stream.write_all(content.as_bytes())?;
    stream.flush()
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pattern() {
        assert_eq!(
            String::from("GET / HTTP/1.1\r\n"),
            create_pattern(HTTPMethod::Get, ""),
        );
        assert_eq!(
            String::from("GET / HTTP/1.1\r\n"),
            create_pattern(HTTPMethod::Get, "/"),
        );
        assert_eq!(
            String::from("POST / HTTP/1.1\r\n"),
            create_pattern(HTTPMethod::Post, "/"),
        );
        assert_eq!(
            String::from("POST /foo/bar HTTP/1.1\r\n"),
            create_pattern(HTTPMethod::Post, "/foo/bar"),
        );
        // simple, not even path validation
        assert_eq!(
            String::from("POST 123_invalid@path-yeah HTTP/1.1\r\n"),
            create_pattern(HTTPMethod::Post, "123_invalid@path-yeah"),
        );
    }

    #[test]
    fn test_http_response_to_string_no_content() {
        assert_eq!(
            String::from("HTTP/1.1 403\r\n\r\n"),
            format!("{}", HTTPResponse::new(403)),
        );
    }

    #[test]
    fn test_http_response_to_string_with_content() {
        assert_eq!(
            String::from("HTTP/1.1 200\r\nContent-Length: 13\r\n\r\nHello, World!"),
            format!("{}", HTTPResponse::new(200).with_content("Hello, World!")),
        );
    }

    // TODO:
    // add tests for handle_connection function :) (use tokio's mockstream for this)
}
