use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

use crate::network::{Method, Request, Response};
use crate::core::Route;


pub struct Server { 
    routes: Vec<Route>
}

impl Server {
    pub fn new() -> Server {
        Server { routes: Vec::new() }    
    }

    pub fn use_route(&mut self, path: &str, method: Method, handler: fn(Request) -> Response) -> Result<(), ()> {
        match Route::parse(path.to_string(), method, handler) { 
            Ok(route) => { self.routes.push(route); Ok(()) },
            Err(_) => Err(()),
        }
    }

    pub fn listen(&self, addr: &str) {
        let listener = match TcpListener::bind(&addr) {
            Err(err) => panic!("{:#?}", err),
            Ok(listener) => listener,
        };

        for incoming in listener.incoming() {
            if let Ok(stream) = incoming {
                let routes = self.routes.clone();
                std::thread::spawn(move || handle_connection(stream, routes));
            };
        };
    }
}

fn handle_connection(mut stream: TcpStream, mut routes: Vec<Route>) {
    let mut buffer = vec![0; 1024];
    
    let request_str = match stream.read(&mut buffer) {
        Ok(size) => String::from_utf8_lossy(&buffer[..size]),
        Err(err) => panic!("{:#?}", err),
    };

    let request = match Request::from_str(&request_str) {
        Err(err) => panic!("{:#?}", err),
        Ok(request) => request,
    };

    for route in routes.iter_mut() {
        let response = route.handle(request);
        stream.write_all(response.to_string().as_bytes()).unwrap();
        break;
    };
}
