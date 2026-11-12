use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const DEFAULT_PORT: u16 = 4852;

fn main() -> std::io::Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let port = arguments
        .iter()
        .position(|argument| argument == "--port")
        .and_then(|index| arguments.get(index + 1))
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let port_value = arguments
        .iter()
        .position(|argument| argument == "--port")
        .map(|index| index + 1);
    let command = arguments
        .iter()
        .enumerate()
        .filter(|(index, argument)| !argument.starts_with("--") && Some(*index) != port_value)
        .map(|(_, argument)| argument.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let command = if command.is_empty() {
        "State".to_owned()
    } else {
        command
    };
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.write_all(format!("{command}\n").as_bytes())?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut response = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => response.extend_from_slice(&buffer[..count]),
            Err(_) => break,
        }
    }
    println!("{}", String::from_utf8_lossy(&response));
    Ok(())
}
