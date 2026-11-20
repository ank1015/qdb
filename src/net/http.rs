//! HTTP/1.1, as much as the API needs: requests with a Content-Length
//! body, keep-alive, answers with a body or in chunks; and a client for
//! tests and tools.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;

/// The most bytes of request line and headers.
pub const MAX_HEAD: usize = 16 << 10;

#[derive(Debug, Clone, Default)]
pub struct Request {
    pub method: String,
    /// The path without the query.
    pub path: String,
    pub query: Vec<(String, String)>,
    /// Names in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub keep_alive: bool,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    pub fn query(&self, name: &str) -> Option<&str> {
        self.query.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

#[derive(Debug)]
pub enum ReadError {
    /// The connection ended before a request began.
    Closed,
    /// Not a request this server can read. The text says why.
    Malformed(String),
    /// The body is larger than allowed (its declared length).
    TooLarge(u64),
    Io(io::Error),
}

impl From<io::Error> for ReadError {
    fn from(e: io::Error) -> ReadError {
        ReadError::Io(e)
    }
}

fn read_line(r: &mut impl BufRead, limit: &mut usize) -> Result<Option<String>, ReadError> {
    let mut line = Vec::new();
    loop {
        let buf = r.fill_buf()?;
        if buf.is_empty() {
            return if line.is_empty() { Ok(None) } else { Err(ReadError::Malformed("the connection ended inside the head".into())) };
        }
        let (take, done) = match memchr::memchr(b'\n', buf) {
            Some(i) => (i + 1, true),
            None => (buf.len(), false),
        };
        if take > *limit {
            return Err(ReadError::Malformed("request head too long".into()));
        }
        *limit -= take;
        line.extend_from_slice(&buf[..take]);
        r.consume(take);
        if done {
            while line.last().is_some_and(|c| *c == b'\n' || *c == b'\r') {
                line.pop();
            }
            return String::from_utf8(line).map(Some).map_err(|_| ReadError::Malformed("request head is not UTF-8".into()));
        }
    }
}

/// Undo %-escapes (and `+` for a space, in a query).
pub fn unescape(s: &str, plus: bool) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                let digit = |c: u8| (c as char).to_digit(16);
                match (digit(b[i + 1]), digit(b[i + 2])) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8);
                        i += 3;
                        continue;
                    }
                    _ => out.push(b'%'),
                }
            }
            b'+' if plus => out.push(b' '),
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Read one request. Bodies longer than `max_body` are refused after the
/// head is read, without reading them.
pub fn read_request(r: &mut impl BufRead, max_body: usize) -> Result<Request, ReadError> {
    let mut limit = MAX_HEAD;
    let mut line = match read_line(r, &mut limit)? {
        None => return Err(ReadError::Closed),
        Some(l) => l,
    };
    // Blank lines before a request are allowed.
    while line.is_empty() {
        line = match read_line(r, &mut limit)? {
            None => return Err(ReadError::Closed),
            Some(l) => l,
        };
    }
    let mut parts = line.split(' ');
    let (Some(method), Some(target), Some(version), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(ReadError::Malformed("bad request line".into()));
    };
    if !version.starts_with("HTTP/1.") {
        return Err(ReadError::Malformed("not HTTP/1.x".into()));
    }
    let mut req = Request { method: method.to_string(), keep_alive: version == "HTTP/1.1", ..Request::default() };
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    req.path = unescape(path, false);
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        req.query.push((unescape(k, true), unescape(v, true)));
    }
    loop {
        let Some(h) = read_line(r, &mut limit)? else { return Err(ReadError::Malformed("the connection ended inside the head".into())) };
        if h.is_empty() {
            break;
        }
        let Some((name, value)) = h.split_once(':') else { return Err(ReadError::Malformed("bad header line".into())) };
        req.headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    if let Some(c) = req.header("connection") {
        let c = c.to_ascii_lowercase();
        if c.contains("close") {
            req.keep_alive = false;
        } else if c.contains("keep-alive") {
            req.keep_alive = true;
        }
    }
    if req.header("transfer-encoding").is_some() {
        return Err(ReadError::Malformed("a chunked request body is not supported; send Content-Length".into()));
    }
    let len = match req.header("content-length") {
        None => 0,
        Some(v) => v.parse::<u64>().map_err(|_| ReadError::Malformed("bad Content-Length".into()))?,
    };
    if len > max_body as u64 {
        return Err(ReadError::TooLarge(len));
    }
    req.body = vec![0; len as usize];
    r.read_exact(&mut req.body)?;
    Ok(req)
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        507 => "Insufficient Storage",
        _ => "Status",
    }
}

/// Write a whole answer.
pub fn write_response(w: &mut impl Write, status: u16, content_type: &str, body: &[u8], keep_alive: bool) -> io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}\r\n",
        status,
        reason(status),
        content_type,
        body.len(),
        if keep_alive { "" } else { "Connection: close\r\n" }
    );
    let mut all = Vec::with_capacity(head.len() + body.len());
    all.extend_from_slice(head.as_bytes());
    all.extend_from_slice(body);
    w.write_all(&all)?;
    w.flush()
}

/// The head of an answer whose body follows in chunks.
pub fn write_chunked_head(w: &mut impl Write, status: u16, content_type: &str, keep_alive: bool) -> io::Result<()> {
    let head = format!("HTTP/1.1 {} {}\r\nContent-Type: {}\r\nTransfer-Encoding: chunked\r\n{}\r\n", status, reason(status), content_type, if keep_alive { "" } else { "Connection: close\r\n" });
    w.write_all(head.as_bytes())?;
    w.flush()
}

pub fn write_chunk(w: &mut impl Write, data: &[u8]) -> io::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let mut all = format!("{:x}\r\n", data.len()).into_bytes();
    all.extend_from_slice(data);
    all.extend_from_slice(b"\r\n");
    w.write_all(&all)?;
    w.flush()
}

pub fn write_last_chunk(w: &mut impl Write) -> io::Result<()> {
    w.write_all(b"0\r\n\r\n")?;
    w.flush()
}

// ---------------------------------------------------------------- client

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// One connection to a server, kept open between requests.
pub struct Client {
    addr: String,
    conn: Option<BufReader<TcpStream>>,
}

impl Client {
    pub fn new(addr: &str) -> Client {
        Client { addr: addr.to_string(), conn: None }
    }

    fn conn(&mut self) -> io::Result<&mut BufReader<TcpStream>> {
        if self.conn.is_none() {
            let s = TcpStream::connect(&self.addr)?;
            s.set_nodelay(true)?;
            self.conn = Some(BufReader::with_capacity(64 << 10, s));
        }
        Ok(self.conn.as_mut().unwrap())
    }

    /// Drop the connection; the next request makes a new one.
    pub fn reset(&mut self) {
        self.conn = None;
    }

    /// The connection, for a test that wants to cut it.
    pub fn stream(&mut self) -> io::Result<&TcpStream> {
        Ok(self.conn()?.get_ref())
    }

    /// Send a request and read the whole answer. Chunks of a chunked
    /// answer are handed to `on_chunk` as they come if it is given, and
    /// gathered into the body otherwise.
    pub fn request_with(&mut self, method: &str, path: &str, key: Option<&str>, body: &[u8], mut on_chunk: Option<&mut dyn FnMut(&[u8])>) -> io::Result<Response> {
        let mut head = format!("{} {} HTTP/1.1\r\nHost: qdb\r\nContent-Length: {}\r\n", method, path, body.len());
        if let Some(k) = key {
            head.push_str(&format!("Authorization: Bearer {}\r\n", k));
        }
        if !body.is_empty() {
            head.push_str("Content-Type: application/json\r\n");
        }
        head.push_str("\r\n");
        let mut all = head.into_bytes();
        all.extend_from_slice(body);
        let result = (|| {
            let c = self.conn()?;
            c.get_mut().write_all(&all)?;
            read_response(c, &mut on_chunk)
        })();
        match &result {
            Ok(r) if r.headers.iter().any(|(n, v)| n == "connection" && v.eq_ignore_ascii_case("close")) => self.conn = None,
            Err(_) => self.conn = None,
            _ => {}
        }
        result
    }

    pub fn request(&mut self, method: &str, path: &str, key: Option<&str>, body: &[u8]) -> io::Result<Response> {
        self.request_with(method, path, key, body, None)
    }
}

fn read_response(r: &mut BufReader<TcpStream>, on_chunk: &mut Option<&mut dyn FnMut(&[u8])>) -> io::Result<Response> {
    let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_string());
    let mut limit = MAX_HEAD;
    let line = read_line(r, &mut limit).map_err(|_| bad("bad answer head"))?.ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "the server closed the connection"))?;
    let status: u16 = line.split(' ').nth(1).and_then(|s| s.parse().ok()).ok_or_else(|| bad("bad status line"))?;
    let mut headers = Vec::new();
    loop {
        let h = read_line(r, &mut limit).map_err(|_| bad("bad answer head"))?.ok_or_else(|| bad("answer ended in its head"))?;
        if h.is_empty() {
            break;
        }
        if let Some((n, v)) = h.split_once(':') {
            headers.push((n.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    let chunked = headers.iter().any(|(n, v)| n == "transfer-encoding" && v.eq_ignore_ascii_case("chunked"));
    let mut body = Vec::new();
    if chunked {
        loop {
            let mut lim = 1024;
            let size = read_line(r, &mut lim).map_err(|_| bad("bad chunk size"))?.ok_or_else(|| bad("answer ended in a chunk"))?;
            let n = usize::from_str_radix(size.split(';').next().unwrap_or("").trim(), 16).map_err(|_| bad("bad chunk size"))?;
            if n == 0 {
                let mut lim = 1024;
                let _ = read_line(r, &mut lim);
                break;
            }
            let mut chunk = vec![0; n];
            r.read_exact(&mut chunk)?;
            let mut crlf = [0u8; 2];
            r.read_exact(&mut crlf)?;
            match on_chunk {
                Some(f) => f(&chunk),
                None => body.extend_from_slice(&chunk),
            }
        }
    } else {
        let len: usize = headers.iter().find(|(n, _)| n == "content-length").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
        body = vec![0; len];
        r.read_exact(&mut body)?;
    }
    Ok(Response { status, headers, body })
}
