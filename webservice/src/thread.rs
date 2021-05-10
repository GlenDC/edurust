//! Minimal implementation of a [ThreadPool](self::ThreadPool),
//! allowing you to run different tasks in parallel,
//! while at the same time remaining in control on the max amount
//! of threads to be used at any given point.
//! 
//! 
//! # Example
//! 
//! ```
//! # use std::thread;
//! # use std::time::{Duration, Instant};
//! # use webservice::thread::Result;
//! use webservice::thread::ThreadPool;
//! 
//! # fn main() -> Result<()> {
//! # let start = Instant::now();
//! println!("create pool and do some work");
//! {
//!     let pool = ThreadPool::new(2)?;
//!     pool.execute(|| {
//!         println!("heavy job #1");
//!         thread::sleep(Duration::from_secs(1));
//!     });
//!     pool.execute(|| {
//!         println!("heavy job #2");
//!         thread::sleep(Duration::from_secs(1));
//!     });
//! }
//! println!("all workers are done, total time since start: {}",  start.elapsed().as_secs());
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::result;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;

use log;

/// `PoolError` is the error used for any errors resulting
/// from creating or using a [ThreadPool](self::ThreadPool).
#[derive(Debug, PartialEq)]
pub struct PoolError {
    /// The kind of error that happened,
    /// allowing you to handle the error appropriately.
    pub kind: PoolErrorKind,
    /// A human readable message for debugging purposes only.
    pub message: &'static str,
}

/// Defines the kind of error that happened related to the creation
/// or usage of a [ThreadPool](self::ThreadPool).
#[derive(Debug, PartialEq)]
pub enum PoolErrorKind {
    /// Indicates that a wrong size was used to create a
    /// [ThreadPool](self::ThreadPool), refer to the
    /// documentation of [ThreadPool::new](self::ThreadPool::new)
    /// to find what size is appropriate.
    InvalidSize
}

/// Result alias type used for all functions within this create which
/// can fail in a recoverable fashion.
pub type Result<T> = result::Result<T, PoolError>;

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = format!("PoolError::{:?}: {}", self.kind, self.message);
        f.write_str(message.as_str())
    }
}

/// A pool of pre-allocated threads ready to execute work.
/// This allows you to put an upper limit of how many threads can be used
/// at any given time.
/// 
/// A useful example is a WebService which limits the amount of concurrent requests
/// it will handle in order to not expose itself to a DDoS attack.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    /// 
    /// The size is the number of threads in the pool.
    /// 
    /// # Errors
    /// 
    /// A [PoolError](self::PoolError) is returned with kind [PoolErrorKind::InvalidSize](self::PoolErrorKind::InvalidSize)
    /// if a size of 0 is given, all strictly positive integers can be used as a valid size up to the max usize value.
    pub fn new(size: usize) -> Result<ThreadPool> {
        if size == 0 {
            return Err(PoolError{
                kind: PoolErrorKind::InvalidSize,
                message: "pool size has to be within the inclusive range of [1, usize::max]",
            });
        }

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Ok(ThreadPool { workers, sender })
    }

    /// Schedule work to be done by one of the pre-allocated threads
    /// of this [ThreadPool](self::ThreadPool). It is undefined
    /// how long the work has to wait prior to actually being executed,
    /// this depends upon many factors including work already scheduled
    /// and the size of the [ThreadPool](self::ThreadPool).
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        log::debug!("Sending terminate message to all workers.");

        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        log::debug!("Shutting down all workers.");

        for worker in &mut self.workers {
            log::debug!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl fmt::Debug for ThreadPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let size = self.workers.len();
        f.debug_struct("ThreadPool")
         .field("size", &size)
         .finish()
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver:  Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    log::debug!("Worker {} got a job; executing.", id);
                    job();
                }
                Message::Terminate => {
                    log::debug!("Worker {} was told to terminate.", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_size_pool() {
        assert_eq!(ThreadPool::new(0).unwrap_err().kind, PoolErrorKind::InvalidSize);
    }

    #[test]
    fn test_valid_size_pool() {
        ThreadPool::new(1).unwrap();
    }
}
