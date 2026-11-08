//! One bounded HTTP/1.1 request to a loopback endpoint, using `std` only.
//!
//! Deliberately narrow: no redirects, no proxies, no TLS, no retries, no chunked
//! transfer decoding, hard byte caps and an explicit timeout. Anything outside
//! that envelope is rejected rather than guessed at.

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::{Duration, Instant};

/// Largest accepted response body.
pub const MAX_RESPONSE_BYTES: usize = 64 * 1024;
/// Largest accepted request body.
pub const MAX_REQUEST_BYTES: usize = 8 * 1024;
/// Largest accepted authorization header file.
pub const MAX_AUTHORIZATION_BYTES: usize = 2 * 1024;
/// Connect/read/write timeout for a single request.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterError {
    /// The endpoint is not a loopback address.
    NotLoopback,
    /// The authorization header is empty, too long or contains a newline.
    BadAuthorization,
    /// The request or response exceeded its declared bound.
    Oversize(&'static str),
    /// No connection could be established or a byte-level operation failed.
    Unavailable(String),
    /// The exchange exceeded the timeout.
    Timeout,
    /// The peer answered with a non-200 status.
    Status(u16),
    /// The response was not the single shape this adapter understands.
    Malformed(&'static str),
}

impl core::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotLoopback => f.write_str("endpoint is not loopback"),
            Self::BadAuthorization => f.write_str("authorization header rejected"),
            Self::Oversize(what) => write!(f, "oversize {what}"),
            Self::Unavailable(detail) => write!(f, "unavailable: {detail}"),
            Self::Timeout => f.write_str("timeout"),
            Self::Status(code) => write!(f, "http status {code}"),
            Self::Malformed(detail) => write!(f, "malformed response: {detail}"),
        }
    }
}

impl std::error::Error for AdapterError {}

/// A pinned loopback endpoint plus its bearer header line.
#[derive(Clone, PartialEq, Eq)]
pub struct Endpoint {
    addr: SocketAddr,
    authorization: String,
    timeout: Duration,
}

impl Endpoint {
    /// Build an endpoint for `127.0.0.1:port`.
    pub fn loopback(port: u16, authorization: impl Into<String>) -> Result<Self, AdapterError> {
        Self::with_addr(
            SocketAddr::from((IpAddr::from([127, 0, 0, 1]), port)),
            authorization,
        )
    }

    /// Build an endpoint for an explicit address; non-loopback is rejected.
    pub fn with_addr(
        addr: SocketAddr,
        authorization: impl Into<String>,
    ) -> Result<Self, AdapterError> {
        if !addr.ip().is_loopback() {
            return Err(AdapterError::NotLoopback);
        }
        let authorization = authorization.into();
        let trimmed = authorization.trim_end_matches(['\r', '\n']).trim();
        if trimmed.is_empty()
            || trimmed.len() > MAX_AUTHORIZATION_BYTES
            || trimmed.contains('\n')
            || trimmed.contains('\r')
        {
            return Err(AdapterError::BadAuthorization);
        }
        // Accept either a complete header line (`Authorization: Bearer ...`) as
        // produced by the local stack, or a bare credential, which is then sent
        // as a bearer header. Exactly one header line is emitted either way.
        let lower = trimmed.to_ascii_lowercase();
        let authorization = if lower.starts_with("authorization:") {
            trimmed.to_owned()
        } else {
            format!("Authorization: Bearer {trimmed}")
        };
        Ok(Self {
            addr,
            authorization,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// Read the bearer header line from a file with a bounded read.
    pub fn from_header_file(port: u16, path: &Path) -> Result<Self, AdapterError> {
        let file = std::fs::File::open(path)
            .map_err(|error| AdapterError::Unavailable(format!("header file: {error}")))?;
        let mut buffer = Vec::new();
        file.take(MAX_AUTHORIZATION_BYTES as u64 + 1)
            .read_to_end(&mut buffer)
            .map_err(|error| AdapterError::Unavailable(format!("header read: {error}")))?;
        if buffer.len() > MAX_AUTHORIZATION_BYTES {
            return Err(AdapterError::Oversize("authorization header"));
        }
        let text = String::from_utf8(buffer).map_err(|_| AdapterError::BadAuthorization)?;
        Self::loopback(port, text)
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    /// Build the exact request bytes. The stored authorization value is already
    /// a complete header line, so it is emitted verbatim and exactly once.
    pub fn build_request(&self, path: &str, body: &str) -> String {
        format!(
            "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\n{authorization}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}",
            host = self.addr.ip(),
            port = self.addr.port(),
            authorization = self.authorization,
            length = body.len(),
        )
    }

    /// Perform one bounded request and return the raw response body.
    pub fn post_json(&self, path: &str, body: &str) -> Result<(Vec<u8>, Duration), AdapterError> {
        if !path.starts_with('/') || path.contains(['\r', '\n', ' ']) {
            return Err(AdapterError::Malformed("request path"));
        }
        if body.len() > MAX_REQUEST_BYTES {
            return Err(AdapterError::Oversize("request body"));
        }
        let request = self.build_request(path, body);

        let started = Instant::now();
        let mut stream = TcpStream::connect_timeout(&self.addr, self.timeout).map_err(|error| {
            if matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) {
                AdapterError::Timeout
            } else {
                AdapterError::Unavailable(error.to_string())
            }
        })?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|error| AdapterError::Unavailable(error.to_string()))?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|error| AdapterError::Unavailable(error.to_string()))?;
        stream
            .write_all(request.as_bytes())
            .map_err(|error| AdapterError::Unavailable(error.to_string()))?;
        stream
            .flush()
            .map_err(|error| AdapterError::Unavailable(error.to_string()))?;

        let mut raw = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => {
                    if raw.len() + count > MAX_RESPONSE_BYTES {
                        return Err(AdapterError::Oversize("response"));
                    }
                    raw.extend_from_slice(&chunk[..count]);
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Err(AdapterError::Timeout);
                }
                Err(error) => return Err(AdapterError::Unavailable(error.to_string())),
            }
        }
        let elapsed = started.elapsed();
        let body = split_response(&raw)?;
        Ok((body.to_vec(), elapsed))
    }
}

/// Split an HTTP/1.1 response into its body, validating the part we rely on.
pub fn split_response(raw: &[u8]) -> Result<&[u8], AdapterError> {
    let boundary = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or(AdapterError::Malformed("no header terminator"))?;
    let (head, rest) = raw.split_at(boundary);
    let body = &rest[4..];
    let head = core::str::from_utf8(head).map_err(|_| AdapterError::Malformed("header utf8"))?;

    let mut lines = head.split("\r\n");
    let status_line = lines
        .next()
        .ok_or(AdapterError::Malformed("missing status line"))?;
    let mut parts = status_line.split(' ');
    let version = parts.next().unwrap_or_default();
    if !version.starts_with("HTTP/1.") {
        return Err(AdapterError::Malformed("status version"));
    }
    let code: u16 = parts
        .next()
        .and_then(|text| text.parse().ok())
        .ok_or(AdapterError::Malformed("status code"))?;

    let mut content_length: Option<usize> = None;
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or(AdapterError::Malformed("header line"))?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "content-length" => {
                let length: usize = value
                    .parse()
                    .map_err(|_| AdapterError::Malformed("content length"))?;
                if content_length.replace(length).is_some() {
                    return Err(AdapterError::Malformed("duplicate content length"));
                }
            }
            "transfer-encoding" => {
                // This adapter does not implement chunked decoding.
                return Err(AdapterError::Malformed("transfer encoding unsupported"));
            }
            _ => {}
        }
    }

    if code != 200 {
        return Err(AdapterError::Status(code));
    }
    if let Some(length) = content_length
        && length != body.len()
    {
        return Err(AdapterError::Malformed("content length mismatch"));
    }
    Ok(body)
}

/// Ensure the body is single-line-safe JSON text we can hand to the parser.
pub fn body_text(body: &[u8]) -> Result<&str, AdapterError> {
    core::str::from_utf8(body).map_err(|_| AdapterError::Malformed("body utf8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_loopback_endpoints() {
        let remote = SocketAddr::from(([10, 0, 0, 5], 1235));
        // `Endpoint` deliberately has no `Debug`: it holds the bearer header.
        match Endpoint::with_addr(remote, "Authorization: Bearer x") {
            Err(AdapterError::NotLoopback) => {}
            _ => panic!("expected a loopback rejection"),
        }
    }

    #[test]
    fn rejects_bad_authorization_headers() {
        assert!(matches!(
            Endpoint::loopback(1, ""),
            Err(AdapterError::BadAuthorization)
        ));
        assert!(matches!(
            Endpoint::loopback(1, "Authorization: a\r\nX-Injected: b"),
            Err(AdapterError::BadAuthorization)
        ));
        // A bare credential is normalized to exactly one header line.
        let bare = Endpoint::loopback(2, "token-value").expect("bare credential");
        let file = Endpoint::loopback(2, "Authorization: Bearer token-value").expect("header");
        assert_eq!(bare.authorization, file.authorization);
        let request = bare.build_request("/v1/rerank", "{}");
        assert_eq!(request.matches("Authorization:").count(), 1, "{request}");
        assert!(!request.contains("Authorization: Authorization:"));
    }

    #[test]
    fn splits_a_well_formed_response() {
        let raw = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 7\r\n\r\n{\"a\":1}";
        assert_eq!(split_response(raw).expect("split"), b"{\"a\":1}");
    }

    #[test]
    fn reports_non_200_status() {
        let raw = b"HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n";
        assert_eq!(split_response(raw), Err(AdapterError::Status(401)));
    }

    #[test]
    fn rejects_chunked_and_mismatched_bodies() {
        let chunked = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n0\r\n\r\n";
        assert!(matches!(
            split_response(chunked),
            Err(AdapterError::Malformed("transfer encoding unsupported"))
        ));
        let mismatched = b"HTTP/1.1 200 OK\r\ncontent-length: 99\r\n\r\n{}";
        assert!(matches!(
            split_response(mismatched),
            Err(AdapterError::Malformed("content length mismatch"))
        ));
    }

    #[test]
    fn rejects_oversize_request_bodies() {
        let endpoint = Endpoint::loopback(1, "Authorization: Bearer x").expect("endpoint");
        let body = "x".repeat(MAX_REQUEST_BYTES + 1);
        assert_eq!(
            endpoint.post_json("/v1/rerank", &body),
            Err(AdapterError::Oversize("request body"))
        );
    }
}
