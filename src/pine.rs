use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use thiserror::Error;

use crate::state::RuntimeState;

const MSG_TITLE: u8 = 0x0B;
const MSG_ID: u8 = 0x0C;
const MSG_UUID: u8 = 0x0D;
const MSG_GAME_VERSION: u8 = 0x0E;
const MSG_STATUS: u8 = 0x0F;

#[derive(Debug, Error)]
pub enum PineError {
    #[error("PINE connection failed: {0}")]
    Connect(#[from] std::io::Error),
    #[error("PINE returned failure")]
    Failure,
    #[error("invalid PINE response")]
    InvalidResponse,
    #[error("invalid UTF-8 in PINE response")]
    Utf8(#[from] std::string::FromUtf8Error),
}

pub struct PineClient {
    stream: TcpStream,
}

impl PineClient {
    pub fn connect(host: &str, port: u16) -> Result<Self, PineError> {
        let address = (host, port).to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "no address")
        })?;

        let stream = TcpStream::connect_timeout(&address, Duration::from_millis(1000))?;
        stream.set_read_timeout(Some(Duration::from_millis(1500)))?;
        stream.set_write_timeout(Some(Duration::from_millis(1500)))?;
        Ok(Self { stream })
    }

    fn command(&mut self, opcode: u8) -> Result<Vec<u8>, PineError> {
        // PINE's frame length includes its four-byte length prefix.
        let length = 5u32.to_le_bytes();
        self.stream.write_all(&length)?;
        self.stream.write_all(&[opcode])?;

        let mut header = [0u8; 4];
        self.stream.read_exact(&mut header)?;
        let total_len = u32::from_le_bytes(header) as usize;
        if !(5..=1024 * 1024).contains(&total_len) {
            return Err(PineError::InvalidResponse);
        }

        // The four-byte length header has already been consumed.
        let payload_len = total_len - 4;
        let mut response = vec![0u8; payload_len];
        self.stream.read_exact(&mut response)?;
        if response.first().copied() != Some(0) {
            return Err(PineError::Failure);
        }
        Ok(response)
    }

    fn text(&mut self, opcode: u8) -> Result<String, PineError> {
        let response = self.command(opcode)?;
        if response.len() < 5 {
            return Err(PineError::InvalidResponse);
        }

        let text_len = u32::from_le_bytes(response[1..5].try_into().unwrap()) as usize;
        if text_len == 0 || 5 + text_len > response.len() {
            return Err(PineError::InvalidResponse);
        }

        let bytes = &response[5..5 + text_len];
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8(bytes[..end].to_vec())?)
    }

    fn status(&mut self) -> Result<u32, PineError> {
        // Reply layout after the length prefix: one result byte, then the
        // u32 emulator status (Running = 0, Paused = 1, Shutdown = 2).
        let response = self.command(MSG_STATUS)?;
        if response.len() < 5 {
            return Err(PineError::InvalidResponse);
        }
        Ok(u32::from_le_bytes(response[1..5].try_into().unwrap()))
    }

    pub fn read_state(&mut self) -> Result<RuntimeState, PineError> {
        let status = self.status()?;

        match status {
            0 | 1 => {}
            2 => return Ok(RuntimeState::Idle),
            _ => return Err(PineError::InvalidResponse),
        }

        let title = self.text(MSG_TITLE).unwrap_or_default().trim().to_string();
        let serial = self.text(MSG_ID).unwrap_or_default().trim().to_string();
        let crc = self.text(MSG_UUID).unwrap_or_default().trim().to_string();
        let version = self
            .text(MSG_GAME_VERSION)
            .unwrap_or_default()
            .trim()
            .to_string();

        if title.is_empty() && serial.is_empty() {
            return Ok(RuntimeState::Bios {
                paused: status == 1,
            });
        }

        Ok(RuntimeState::Game {
            title,
            serial,
            crc,
            version,
            paused: status == 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::PineClient;
    use crate::state::RuntimeState;

    fn send_response(stream: &mut std::net::TcpStream, payload: &[u8]) {
        let total_len = (4 + payload.len()) as u32;
        stream.write_all(&total_len.to_le_bytes()).unwrap();
        stream.write_all(payload).unwrap();
    }

    fn send_text(stream: &mut std::net::TcpStream, value: &str) {
        let bytes = value.as_bytes();
        let mut payload = Vec::with_capacity(5 + bytes.len() + 1);
        payload.push(0);
        payload.extend_from_slice(&((bytes.len() + 1) as u32).to_le_bytes());
        payload.extend_from_slice(bytes);
        payload.push(0);
        send_response(stream, &payload);
    }

    #[test]
    fn reads_game_state_from_pine() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            for (index, expected_opcode) in [0x0F_u8, 0x0B, 0x0C, 0x0D, 0x0E].iter().enumerate() {
                let mut header = [0u8; 4];
                stream.read_exact(&mut header).unwrap();
                assert_eq!(u32::from_le_bytes(header), 5);

                let mut opcode = [0u8; 1];
                stream.read_exact(&mut opcode).unwrap();
                assert_eq!(opcode[0], *expected_opcode);

                match index {
                    0 => send_response(&mut stream, &[0, 0, 0, 0, 0]),
                    1 => send_text(&mut stream, "Grand Theft Auto: San Andreas"),
                    2 => send_text(&mut stream, "SLUS-20946"),
                    3 => send_text(&mut stream, "1234abcd"),
                    4 => send_text(&mut stream, "1.00"),
                    _ => unreachable!(),
                }
            }
        });

        let mut client = PineClient::connect("127.0.0.1", port).unwrap();
        let state = client.read_state().unwrap();

        assert_eq!(
            state,
            RuntimeState::Game {
                title: "Grand Theft Auto: San Andreas".into(),
                serial: "SLUS-20946".into(),
                crc: "1234abcd".into(),
                version: "1.00".into(),
                paused: false,
            }
        );

        server.join().unwrap();
    }
    #[test]
    fn pine_status_values_are_stable() {
        assert_eq!(0u32, 0); // Running
        assert_eq!(1u32, 1); // Paused
        assert_eq!(2u32, 2); // Shutdown
    }

    fn send_status(stream: &mut std::net::TcpStream, status: u32) {
        let mut payload = vec![0u8];
        payload.extend_from_slice(&status.to_le_bytes());
        send_response(stream, &payload);
    }

    fn send_fail(stream: &mut std::net::TcpStream) {
        send_response(stream, &[0xFF]);
    }

    fn read_request(stream: &mut std::net::TcpStream) -> u8 {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).unwrap();
        assert_eq!(u32::from_le_bytes(header), 5);
        let mut opcode = [0u8; 1];
        stream.read_exact(&mut opcode).unwrap();
        opcode[0]
    }

    #[test]
    fn shutdown_status_maps_to_idle() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            assert_eq!(read_request(&mut stream), 0x0F);
            send_status(&mut stream, 2);
        });

        let mut client = PineClient::connect("127.0.0.1", port).unwrap();
        assert_eq!(client.read_state().unwrap(), RuntimeState::Idle);
        server.join().unwrap();
    }

    #[test]
    fn paused_status_maps_to_paused_game() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            assert_eq!(read_request(&mut stream), 0x0F);
            send_status(&mut stream, 1);
            assert_eq!(read_request(&mut stream), 0x0B);
            send_text(&mut stream, "Gran Turismo 4");
            assert_eq!(read_request(&mut stream), 0x0C);
            send_text(&mut stream, "SCES-51719");
            assert_eq!(read_request(&mut stream), 0x0D);
            send_text(&mut stream, "77e61c8a");
            assert_eq!(read_request(&mut stream), 0x0E);
            send_text(&mut stream, "1.00");
        });

        let mut client = PineClient::connect("127.0.0.1", port).unwrap();
        let state = client.read_state().unwrap();
        assert_eq!(
            state,
            RuntimeState::Game {
                title: "Gran Turismo 4".into(),
                serial: "SCES-51719".into(),
                crc: "77e61c8a".into(),
                version: "1.00".into(),
                paused: true,
            }
        );
        server.join().unwrap();
    }

    #[test]
    fn running_vm_without_metadata_maps_to_bios() {
        // PCSX2 fails the title/serial requests when the VM has no game
        // metadata, which is what the PS2 system menu looks like.
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            assert_eq!(read_request(&mut stream), 0x0F);
            send_status(&mut stream, 0);
            for expected in [0x0B_u8, 0x0C, 0x0D, 0x0E] {
                assert_eq!(read_request(&mut stream), expected);
                send_fail(&mut stream);
            }
        });

        let mut client = PineClient::connect("127.0.0.1", port).unwrap();
        assert_eq!(
            client.read_state().unwrap(),
            RuntimeState::Bios { paused: false }
        );
        server.join().unwrap();
    }

    #[test]
    fn rejects_implausible_length_prefix() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_request(&mut stream);
            // A length prefix below the minimum frame size is malformed.
            stream.write_all(&2u32.to_le_bytes()).unwrap();
            stream.write_all(&[0u8; 8]).unwrap();
        });

        let mut client = PineClient::connect("127.0.0.1", port).unwrap();
        assert!(client.read_state().is_err());
        server.join().unwrap();
    }

    #[test]
    fn survives_connection_reset_mid_reply() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            // Drop the connection without answering, like PCSX2 closing.
            drop(stream);
        });

        let mut client = PineClient::connect("127.0.0.1", port).unwrap();
        assert!(client.read_state().is_err());
        server.join().unwrap();
    }

    #[test]
    fn handles_sequential_states_on_one_connection() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            // First poll: PCSX2 open, no VM.
            assert_eq!(read_request(&mut stream), 0x0F);
            send_status(&mut stream, 2);

            // Second poll: game running.
            assert_eq!(read_request(&mut stream), 0x0F);
            send_status(&mut stream, 0);
            assert_eq!(read_request(&mut stream), 0x0B);
            send_text(&mut stream, "Kingdom Hearts");
            assert_eq!(read_request(&mut stream), 0x0C);
            send_text(&mut stream, "SLUS-20370");
            assert_eq!(read_request(&mut stream), 0x0D);
            send_text(&mut stream, "c90c8f3d");
            assert_eq!(read_request(&mut stream), 0x0E);
            send_text(&mut stream, "1.00");
        });

        let mut client = PineClient::connect("127.0.0.1", port).unwrap();
        assert_eq!(client.read_state().unwrap(), RuntimeState::Idle);
        match client.read_state().unwrap() {
            RuntimeState::Game { title, serial, .. } => {
                assert_eq!(title, "Kingdom Hearts");
                assert_eq!(serial, "SLUS-20370");
            }
            other => panic!("expected game state, got {other:?}"),
        }
        server.join().unwrap();
    }
}
