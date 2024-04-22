use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;

use crate::core::path::Error as PathError;
use crate::core::{Middleware, Path, Pipeline, Route};
use crate::core::{MiddlewareHandler, RouteHandler};
use crate::network::{Method, Request};

pub enum Error {
    Path(PathError),
}

pub struct Server {
    pipeline: Vec<Pipeline>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pipeline: Vec::new(),
        }
    }

    /// # Errors
    ///
    /// Returns `Err` if the string is not a valid path
    pub fn use_route(
        &mut self,
        path_str: &str,
        method: Method,
        handler: RouteHandler,
    ) -> Result<(), Error> {
        match Path::parse(path_str) {
            Ok(path) => {
                let route = Route::new(path, method, handler);
                self.pipeline.push(Pipeline::Route(route));
                Ok(())
            }
            Err(e) => Err(Error::Path(e)),
        }
    }

    /// # Errors
    ///
    /// Returns `Err` if the string is not a valid path
    pub fn use_static_middleware(
        &mut self,
        path_str: &str,
        handler: MiddlewareHandler,
    ) -> Result<(), Error> {
        self.append_middleware(path_str, handler, true)
    }

    /// # Errors
    ///
    /// Returns `Err` if the string is not a valid path
    pub fn use_middleware(
        &mut self,
        path_str: &str,
        handler: MiddlewareHandler,
    ) -> Result<(), Error> {
        self.append_middleware(path_str, handler, false)
    }

    fn append_middleware(
        &mut self,
        path_str: &str,
        handler: MiddlewareHandler,
        is_static: bool,
    ) -> Result<(), Error> {
        match Path::parse(path_str) {
            Ok(path) => {
                let middleware = Middleware::new(path, handler, is_static);
                self.pipeline.push(Pipeline::Middleware(middleware));
                Ok(())
            }
            Err(e) => Err(Error::Path(e)),
        }
    }

    pub fn listen(&self, addr: &str) {
        let listener = match TcpListener::bind(addr) {
            Err(err) => todo!("{err:#?}"),
            Ok(listener) => listener,
        };

        for incoming in listener.incoming() {
            if let Ok(stream) = incoming {
                let pipeline = self.pipeline.clone();
                std::thread::spawn(move || handle_connection(stream, pipeline));
            };
        }
    }
}

fn handle_connection(mut stream: TcpStream, pipeline: Vec<Pipeline>) {
    let mut buffer = vec![0; 1024];

    let request_str = match stream.read(&mut buffer) {
        Ok(size) => String::from_utf8_lossy(&buffer[..size]),
        Err(err) => todo!("Request error !"),
    };

    let mut request = match Request::from_str(&request_str) {
        Err(err) => todo!("Request error !"),
        Ok(request) => request,
    };

    for pipe in pipeline {
        if let Pipeline::Middleware(middleware) = pipe {
            if !middleware.compare(request.path()) {
                continue;
            }

            match middleware.handle(request) {
                Ok(modified_request) => {
                    request = modified_request;
                    continue;
                }
                Err(response) => {
                    stream
                        .write_all(response.to_string().as_bytes())
                        .unwrap_or(todo!("Log and retry ?"));
                    break;
                }
            }
        };

        if let Pipeline::Route(route) = pipe {
            if route.method() != request.method() {
                continue;
            }

            if !route.compare(request.path()) {
                continue;
            };

            let response = route.handle(request);
            stream
                .write_all(response.to_string().as_bytes())
                .unwrap_or(todo!("Log and retry ?"));
            break;
        };
    }
}
