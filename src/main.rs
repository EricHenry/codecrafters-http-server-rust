use std::{
    collections::BTreeMap,
    fmt::Display,
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
                let request = Request::try_from(&buffer[..n]).unwrap();
                let response = process_request(&request);
                let r = response.to_string();
                let _ = stream.write_all(r.as_bytes());
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn process_request(request: &Request) -> Response {
    if request.path == "/" {
        Response::new()
    } else if request.path.contains("/echo/") {
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
    } else if request.path == "/user-agent" {
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
    } else {
        let mut response = Response::new();
        response.status_code = StatusCode::NotFound;
        response
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

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            Version::Http1_1 => "HTTP/1.1",
        };

        write!(f, "{}", v)
    }
}

#[allow(dead_code)]
struct Request {
    method: Method,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    version: Version,
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
            let header_key = h[0];
            let header_value = h[1];

            headers.insert(header_key.to_string(), header_value.to_string());
        }

        Ok(Self {
            method,
            path,
            version,
            headers,
        })
    }
}

#[allow(dead_code)]
enum StatusCode {
    Ok,
    NotFound,
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            StatusCode::Ok => "200 OK",
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
}

impl Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = match self {
            ContentType::TextPlain => "text/plain",
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
    use super::*;

    #[test]
    fn root_path() {
        let request = Request {
            method: Method::Get,
            path: "/".to_owned(),
            headers: BTreeMap::new(),
            version: Version::Http1_1,
        };

        let response = process_request(&request);
        let rs = response.to_string();

        assert_eq!(rs.as_bytes(), b"HTTP/1.1 200 OK\r\n\r\n")
    }
    #[test]
    fn not_found() {
        let request = Request {
            method: Method::Get,
            path: "/bad".to_owned(),
            headers: BTreeMap::new(),
            version: Version::Http1_1,
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
    fn parse_header() {
        let request = Request {
            method: Method::Get,
            path: "/user-agent".to_owned(),
            headers: BTreeMap::from_iter([("User-Agent".to_owned(), "curl/7.64.1".to_owned())]),
            version: Version::Http1_1,
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
