# webservice

Based on the "webservice" Rust Book project as found at:
<https://doc.rust-lang.org/book/ch20-00-final-project-a-web-server.html>

## Extra Tasks

- [x] Add more documentation to ThreadPool and its public methods.
- [x] Add signal handling to graceful handle such quit
- [x] Add tests of the library’s ThreadPool functionality.
- [x] Add decent logging support.
- [ ] Add tests of the library’s HTTPServer functionality.
- [x] Change calls to unwrap to more robust error handling.
- [x] Move web server logic into library code;
- [x] Use ThreadPool to perform some task other than serving web requests.
- [x] Find a thread pool crate on crates.io and implement a similar web server using the crate instead. Then compare its API and robustness to the thread pool we implemented.
- [x] Fix bug in custom thread pool implementation which blocks stopping the cli app.
- [ ] Learn to use the debugger for Rust in VSCode.
- [x] Enable CI testing (GitHub workflow).
