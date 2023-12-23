use std::{
    io::{Read, Write},
    net::TcpListener,
};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0; 1028];
                let n = stream.read(&mut buffer).unwrap();
                let request = Request::from_slice(&buffer[..n]).unwrap();

                if request.path == "/" {
                    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
                } else {
                    let _ = stream.write_all(b"HTTP/1.1 404 NOT FOUND\r\n\r\n");
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

enum Method {
    Get,
    Post,
    Put,
    Delete,
    Options,
}

enum Version {
    Http1_1,
}

#[allow(dead_code)]
struct Request {
    method: Method,
    pub path: String,
    version: Version,
}

impl Request {
    fn from_slice(bytes: &[u8]) -> std::io::Result<Self> {
        let request_string = String::from_utf8_lossy(bytes);
        let lines: Vec<&str> = request_string.split("\r\n").collect();
        let first_line: Vec<&str> = lines[0].split(' ').collect();
        let method = match first_line[0] {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            "OPTIONS" => Method::Options,
            unknown => {
                unreachable!("got unknown method: {unknown}");
            }
        };
        let path = first_line[1].to_owned();
        let version = match first_line[2] {
            "HTTP/1.1" => Version::Http1_1,
            unknown => {
                unreachable!("got unknown version: {unknown}");
            }
        };

        Ok(Self {
            method,
            path,
            version,
        })
    }
}
