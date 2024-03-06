use std::{
    collections::BTreeMap,
    env,
    fmt::Display,
    fs::{self, File},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::Mutex,
    thread,
};

use itertools::Itertools;

static SEARCH_DIRECTORY: Mutex<Option<String>> = Mutex::new(None);

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    parse_args();

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_connection(stream);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn parse_args() {
    let args: Vec<String> = env::args().collect();
    if let (Some(flag), Some(dir)) = (args.get(1), args.get(2)) {
        if flag == "--directory" {
            if let Ok(mut search_dir) = SEARCH_DIRECTORY.lock() {
                *search_dir = Some(dir.to_owned());
            }
        }
    }
}

fn handle_connection(mut tcp: TcpStream) {
    let mut buffer = [0; 1028];
    let n = tcp.read(&mut buffer).unwrap();
    let request = Request::try_from(&buffer[..n]).unwrap();
    let response = process_request(&request);
    let r = response.to_string();
    let _ = tcp.write_all(r.as_bytes());
}

fn process_request(request: &Request) -> Response {
    match request.method {
        Method::Get => {
            if request.path == "/" {
                Response::new()
            } else if request.path.contains("/echo/") {
                get_echo(request)
            } else if request.path == "/user-agent" {
                get_user_agent(request)
            } else if request.path.contains("/files/") {
                get_file(request)
            } else {
                not_found()
            }
        }
        Method::Post => {
            if request.path.contains("/files/") {
                post_file(request)
            } else {
                not_found()
            }
        }
        _ => not_found(),
    }
}

fn post_file(req: &Request) -> Response {
    let Some((_, filename)) = req.path.split_once("/files/") else {
        return not_found();
    };

    let sd = SEARCH_DIRECTORY.lock().ok().and_then(|sd| sd.clone());

    let Some(search_dir) = sd else {
        return not_found();
    };

    let Some(body) = &req.body else {
        return not_found();
    };

    let file_path = format!("{search_dir}/{filename}");
    let path = Path::new(&file_path);
    let Ok(mut file) = File::create(path) else {
        return not_found();
    };
    if let Err(e) = file.write_all(body.as_bytes()) {
        eprintln!("failed to write body to file: {e}");
    }

    Response {
        status_code: StatusCode::Created,
        ..Default::default()
    }
}

fn get_echo(request: &Request) -> Response {
    let segments: Vec<&str> = request.path.split("/echo/").collect();
    let body = segments[1].to_owned();

    let mut r = Response::new();
    let mut headers = BTreeMap::new();
    headers.insert(
        HeaderKey::ContentType,
        HeaderValue::ContentType(ContentType::TextPlain),
    );
    headers.insert(
        HeaderKey::ContentLength,
        HeaderValue::ContentLength(body.len()),
    );

    r.headers = headers;
    r.body = Some(body);

    r
}

fn get_user_agent(request: &Request) -> Response {
    let body = request.headers.get("User-Agent").unwrap();

    let mut r = Response::new();
    let mut headers = BTreeMap::new();
    headers.insert(
        HeaderKey::ContentType,
        HeaderValue::ContentType(ContentType::TextPlain),
    );
    headers.insert(
        HeaderKey::ContentLength,
        HeaderValue::ContentLength(body.len()),
    );

    r.headers = headers;
    r.body = Some(body.clone());

    r
}

fn not_found() -> Response {
    let mut response = Response::new();
    response.status_code = StatusCode::NotFound;
    response
}

fn get_file(request: &Request) -> Response {
    let segments: Vec<&str> = request.path.split("/files/").collect();
    let filename = segments[1];

    let sd = {
        if let Ok(search_dir) = SEARCH_DIRECTORY.lock() {
            search_dir.clone()
        } else {
            None
        }
    };

    let Some(search_dir) = sd else {
        return not_found();
    };

    let file_path = format!("{}/{}", search_dir, filename);
    let path = Path::new(&file_path);
    if !path.exists() {
        return not_found();
    }

    // TODO: Read file
    let body = fs::read_to_string(path).expect("to read file");
    let mut r = Response::new();
    let mut headers = BTreeMap::new();
    headers.insert(
        HeaderKey::ContentType,
        HeaderValue::ContentType(ContentType::ApplicationOctetStream),
    );
    headers.insert(
        HeaderKey::ContentLength,
        HeaderValue::ContentLength(body.len()),
    );

    r.headers = headers;
    r.body = Some(body);

    r
}

#[derive(Debug, PartialEq, Eq)]
enum Method {
    Get,
    Post,
    Put,
    Delete,
    Options,
}

#[derive(Debug, PartialEq, Eq)]
enum Version {
    Http1_1,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            Version::Http1_1 => "HTTP/1.1",
        };

        write!(f, "{}", v)
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
struct Request {
    method: Method,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    version: Version,
    pub body: Option<String>,
}

enum RequestToken {
    StartLine(String),
    Header(String),
    Body(String),
    Unknown(String),
}

fn parse_request(bytes: &[u8]) -> Vec<RequestToken> {
    let mut tokens = Vec::new();
    let headers_done = false;

    let request_string = String::from_utf8_lossy(bytes);
    for (i, line) in request_string.split("\r\n").enumerate() {
        if i == 0 {
            tokens.push(RequestToken::StartLine(line.to_owned()));
        } else if !headers_done {
        }
    }

    tokens
}

impl TryFrom<&[u8]> for Request {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let request_string = String::from_utf8_lossy(value);
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

        let mut headers = BTreeMap::new();
        for header in &lines[1..] {
            let h = header.split(": ").collect::<Vec<&str>>();
            if h.len() != 2 {
                break;
            }
            let header_key = h[0];
            let header_value = h[1];

            headers.insert(header_key.to_string(), header_value.to_string());
        }

        let mut body = None;

        if let Some((empty_line_index, _)) = lines.iter().find_position(|l| l.is_empty()) {
            // if the last line is not empty we have a body
            if empty_line_index != (lines.len() - 1) {
                let mut body_str = String::new();
                let body_start_index = empty_line_index + 1;

                for &b in &lines[body_start_index..] {
                    body_str.push_str(b);
                }

                body = Some(body_str);
            }
        }

        Ok(Self {
            method,
            path,
            version,
            headers,
            body,
        })
    }
}

#[allow(dead_code)]
enum StatusCode {
    Ok,
    Created,
    NotFound,
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            StatusCode::Ok => "200 OK",
            StatusCode::Created => "201 CREATED",
            StatusCode::NotFound => "404 NOT FOUND",
        };

        write!(f, "{}", s)
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
enum HeaderKey {
    ContentType,
    ContentLength,
}

impl Display for HeaderKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let k = match self {
            HeaderKey::ContentType => "Content-Type",
            HeaderKey::ContentLength => "Content-Length",
        };
        write!(f, "{}", k)
    }
}

#[allow(dead_code)]
enum HeaderValue {
    ContentType(ContentType),
    ContentLength(usize),
}

impl Display for HeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            HeaderValue::ContentType(ct) => ct.to_string(),
            HeaderValue::ContentLength(l) => l.to_string(),
        };
        write!(f, "{}", v)
    }
}

#[allow(dead_code)]
enum ContentType {
    TextPlain,
    ApplicationOctetStream,
}

impl Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = match self {
            ContentType::TextPlain => "text/plain",
            ContentType::ApplicationOctetStream => "application/octet-stream",
        };

        write!(f, "{}", c)
    }
}

#[allow(dead_code)]
struct Response {
    pub version: Version,
    pub status_code: StatusCode,
    pub headers: BTreeMap<HeaderKey, HeaderValue>,
    pub body: Option<String>,
}

impl Response {
    fn new() -> Self {
        Self::default()
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            version: Version::Http1_1,
            status_code: StatusCode::Ok,
            headers: BTreeMap::new(),
            body: None,
        }
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lines: Vec<String> = vec![];

        let status_line = format!("{} {}", self.version, self.status_code);
        lines.push(status_line);

        for (k, v) in &self.headers {
            let header = format!("{}: {}", k, v);
            lines.push(header);
        }

        if let Some(body) = &self.body {
            lines.push(format!("\n{body}"));
        }

        let r = lines.join("\r\n");
        write!(f, "{}\r\n\r\n", r)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn root_path() {
        let request = Request {
            method: Method::Get,
            path: "/".to_owned(),
            headers: BTreeMap::new(),
            version: Version::Http1_1,
            body: None,
        };

        let response = process_request(&request);
        let rs = response.to_string();

        assert_eq!(rs.as_bytes(), b"HTTP/1.1 200 OK\r\n\r\n")
    }

    #[test]
    fn parse_body() {
        let buffer = b"POST /post HTTP/1.1\r\nHost: localhost:4221\r\nContent-Type: text/plain\r\n\nhello there\r\n\r\n";
        let actual = Request::try_from(&buffer[..]).unwrap();

        let expected = Request {
            method: Method::Post,
            path: "/post".to_owned(),
            headers: BTreeMap::from_iter([
                ("Host".to_owned(), "localhost:4221".to_owned()),
                ("Content-Type".to_owned(), "text/plain".to_owned()),
            ]),
            version: Version::Http1_1,
            body: Some("hello there".to_owned()),
        };

        assert_eq!(actual, expected)
    }

    #[test]
    fn not_found() {
        let request = Request {
            method: Method::Get,
            path: "/bad".to_owned(),
            headers: BTreeMap::new(),
            version: Version::Http1_1,
            body: None,
        };

        let response = process_request(&request);
        let rs = response.to_string();

        assert_eq!(rs.as_bytes(), b"HTTP/1.1 404 NOT FOUND\r\n\r\n")
    }

    #[test]
    fn echo() {
        let request = Request {
            method: Method::Get,
            path: "/echo/hello".to_owned(),
            headers: BTreeMap::new(),
            version: Version::Http1_1,
            body: None,
        };

        let response = process_request(&request);
        let rs = response.to_string();
        println!("actual: {rs}");

        let expected =
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\nhello\r\n\r\n";
        println!("expeted: {expected}");
        assert_eq!(rs.as_bytes(), expected.as_bytes())
    }

    #[test]
    fn parse_request() {
        let buffer = b"GET / HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: Go-http-client/1.1\r\nAccept-Encoding: gzip\r\n\r\n";
        let request = Request::try_from(&buffer[..buffer.len()]).unwrap();

        let expected = Request {
            method: Method::Get,
            path: "/".to_owned(),
            headers: BTreeMap::from_iter([
                ("User-Agent".to_owned(), "Go-http-client/1.1".to_owned()),
                ("Host".to_owned(), "localhost:4221".to_owned()),
                ("Accept-Encoding".to_owned(), "gzip".to_owned()),
            ]),
            version: Version::Http1_1,
            body: None,
        };

        assert_eq!(request, expected)
    }

    #[test]
    fn parse_header() {
        let request = Request {
            method: Method::Get,
            path: "/user-agent".to_owned(),
            headers: BTreeMap::from_iter([
                ("User-Agent".to_owned(), "curl/7.64.1".to_owned()),
                ("Host".to_owned(), "localhost:4221".to_owned()),
                ("Accept-Encoding".to_owned(), "gzip".to_owned()),
            ]),
            version: Version::Http1_1,
            body: None,
        };

        let response = process_request(&request);
        let rs = response.to_string();
        println!("actual: {rs}");

        let expected =
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 11\r\n\ncurl/7.64.1\r\n\r\n";
        println!("expeted: {expected}");
        assert_eq!(rs.as_bytes(), expected.as_bytes())
    }
}
