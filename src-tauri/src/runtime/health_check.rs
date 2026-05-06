use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::{
    models::{HealthCheckConfig, HttpHealthCheckConfig, TcpHealthCheckConfig},
    runtime::{RuntimeError, RuntimeResult},
};

pub fn execute_health_check(config: &HealthCheckConfig) -> RuntimeResult<()> {
    match config.normalized() {
        HealthCheckConfig::Http(http) => execute_http_health_check(&http),
        HealthCheckConfig::Tcp(tcp) => execute_tcp_health_check(&tcp),
    }
}

fn execute_http_health_check(config: &HttpHealthCheckConfig) -> RuntimeResult<()> {
    let parsed = ParsedHttpUrl::parse(&config.url)?;
    let mut stream = connect_with_timeout(&parsed.host, parsed.port, config.timeout_ms)?;
    stream
        .set_read_timeout(Some(Duration::from_millis(config.timeout_ms)))
        .map_err(|error| {
            RuntimeError::new(format!("Failed to set health check timeout: {error}"))
        })?;
    stream
        .set_write_timeout(Some(Duration::from_millis(config.timeout_ms)))
        .map_err(|error| {
            RuntimeError::new(format!("Failed to set health check timeout: {error}"))
        })?;

    let method = if config.method.trim().is_empty() {
        "GET"
    } else {
        config.method.as_str()
    };
    let mut request = format!(
        "{method} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        parsed.path_and_query, parsed.host_header
    );
    if let Some(headers) = config.headers.as_ref() {
        for (key, value) in headers {
            request.push_str(&format!("{key}: {value}\r\n"));
        }
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .map_err(|error| RuntimeError::new(format!("HTTP health check request failed: {error}")))?;

    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|error| {
        RuntimeError::new(format!("HTTP health check response failed: {error}"))
    })?;

    let (status_code, body) = parse_http_response(&response)?;
    if !config.expected_status_codes.contains(&status_code) {
        return Err(RuntimeError::new(format!(
            "HTTP health check returned status {status_code}"
        )));
    }
    if let Some(contains_text) = config.contains_text.as_ref() {
        if !body.contains(contains_text) {
            return Err(RuntimeError::new(format!(
                "HTTP health check body does not contain '{contains_text}'"
            )));
        }
    }

    Ok(())
}

fn execute_tcp_health_check(config: &TcpHealthCheckConfig) -> RuntimeResult<()> {
    connect_with_timeout(&config.host, config.port, config.timeout_ms).map(|_| ())
}

fn connect_with_timeout(host: &str, port: u16, timeout_ms: u64) -> RuntimeResult<TcpStream> {
    let timeout = Duration::from_millis(timeout_ms);
    let address = resolve_socket_addr(host, port)?;

    TcpStream::connect_timeout(&address, timeout)
        .map_err(|error| RuntimeError::new(format!("TCP health check failed: {error}")))
}

fn resolve_socket_addr(host: &str, port: u16) -> RuntimeResult<SocketAddr> {
    let mut addresses = (host, port).to_socket_addrs().map_err(|error| {
        RuntimeError::new(format!("Failed to resolve '{host}:{port}': {error}"))
    })?;

    addresses.next().ok_or_else(|| {
        RuntimeError::new(format!("No socket addresses resolved for '{host}:{port}'"))
    })
}

fn parse_http_response(response: &str) -> RuntimeResult<(u16, &str)> {
    let (head, body) = response
        .split_once("\r\n\r\n")
        .or_else(|| response.split_once("\n\n"))
        .ok_or_else(|| RuntimeError::new("Invalid HTTP response"))?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| RuntimeError::new("HTTP response is missing a status line"))?;
    let mut parts = status_line.split_whitespace();
    let _ = parts
        .next()
        .ok_or_else(|| RuntimeError::new("HTTP response is missing protocol"))?;
    let status_code = parts
        .next()
        .ok_or_else(|| RuntimeError::new("HTTP response is missing status code"))?
        .parse::<u16>()
        .map_err(|error| RuntimeError::new(format!("Invalid HTTP status code: {error}")))?;

    Ok((status_code, body))
}

struct ParsedHttpUrl {
    host: String,
    host_header: String,
    port: u16,
    path_and_query: String,
}

impl ParsedHttpUrl {
    fn parse(url: &str) -> RuntimeResult<Self> {
        let trimmed = url.trim();
        let Some(without_scheme) = trimmed.strip_prefix("http://") else {
            return Err(RuntimeError::new(format!(
                "Only http:// URLs are supported in Sprint 4. Invalid URL: {trimmed}"
            )));
        };
        let (host_port, path_and_query) = match without_scheme.split_once('/') {
            Some((host_port, path)) => (host_port, format!("/{path}")),
            None => (without_scheme, "/".into()),
        };
        if host_port.trim().is_empty() {
            return Err(RuntimeError::new("HTTP health check URL is missing a host"));
        }

        let (host, port) = match host_port.rsplit_once(':') {
            Some((host, port_text)) if !port_text.contains(']') => {
                let port = port_text
                    .parse::<u16>()
                    .map_err(|error| RuntimeError::new(format!("Invalid URL port: {error}")))?;
                (host.to_owned(), port)
            }
            _ => (host_port.to_owned(), 80),
        };

        Ok(Self {
            host: host.clone(),
            host_header: if port == 80 {
                host
            } else {
                format!("{host}:{port}")
            },
            port,
            path_and_query,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use crate::models::{HealthCheckConfig, HttpHealthCheckConfig, TcpHealthCheckConfig};

    use super::{execute_health_check, ParsedHttpUrl};

    #[test]
    fn parses_http_urls_with_ports() {
        let parsed =
            ParsedHttpUrl::parse("http://127.0.0.1:1420/health?ready=1").expect("url should parse");

        assert_eq!(parsed.host, "127.0.0.1");
        assert_eq!(parsed.port, 1420);
        assert_eq!(parsed.path_and_query, "/health?ready=1");
    }

    #[test]
    fn executes_http_health_check_against_local_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("server should accept");
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\nhealthy")
                .expect("response should write");
        });

        let result = execute_health_check(&HealthCheckConfig::Http(HttpHealthCheckConfig {
            enabled: true,
            interval_ms: 1_000,
            timeout_ms: 1_000,
            grace_period_ms: 0,
            success_threshold: 1,
            failure_threshold: 1,
            url: format!("http://{address}/health"),
            method: "GET".into(),
            expected_status_codes: vec![200],
            headers: None,
            contains_text: Some("healthy".into()),
        }));

        assert!(result.is_ok());
    }

    #[test]
    fn executes_tcp_health_check_against_local_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");
        thread::spawn(move || {
            let _ = listener.accept();
        });

        let result = execute_health_check(&HealthCheckConfig::Tcp(TcpHealthCheckConfig {
            enabled: true,
            interval_ms: 1_000,
            timeout_ms: 1_000,
            grace_period_ms: 0,
            success_threshold: 1,
            failure_threshold: 1,
            host: "127.0.0.1".into(),
            port: address.port(),
        }));

        assert!(result.is_ok());
    }
}
