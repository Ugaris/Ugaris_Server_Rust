use std::{fmt, net::SocketAddr};

use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tracing::{debug, info};
use ugaris_protocol::{
    encode_tick_frame,
    login::{LoginBlock, LOGIN_BLOCK_SIZE},
    ClientAction, ClientCommandDecoder,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum SessionEvent {
    Connected {
        id: SessionId,
        peer_addr: SocketAddr,
        commands: mpsc::Sender<SessionCommand>,
    },
    Login {
        id: SessionId,
        login: LoginBlock,
    },
    Action {
        id: SessionId,
        command_kind: u8,
        action: ClientAction,
    },
    Disconnected {
        id: SessionId,
    },
}

#[derive(Debug)]
pub enum SessionCommand {
    Send(BytesMut),
    Disconnect,
}

pub async fn run_session(
    id: SessionId,
    mut socket: TcpStream,
    peer_addr: SocketAddr,
    events: mpsc::Sender<SessionEvent>,
    command_tx: mpsc::Sender<SessionCommand>,
    mut commands: mpsc::Receiver<SessionCommand>,
) -> anyhow::Result<()> {
    info!(%id, %peer_addr, "client connected");
    events
        .send(SessionEvent::Connected {
            id,
            peer_addr,
            commands: command_tx,
        })
        .await
        .ok();

    let mut decoder = ClientCommandDecoder::default();
    let mut buf = [0_u8; 1024];
    let mut login_buffer = BytesMut::with_capacity(LOGIN_BLOCK_SIZE);
    let mut logged_in = false;

    loop {
        tokio::select! {
            read = socket.read(&mut buf) => {
                let read = read?;
                if read == 0 {
                    break;
                }

                let mut unread = &buf[..read];
                if !logged_in {
                    let needed = LOGIN_BLOCK_SIZE.saturating_sub(login_buffer.len());
                    let take = needed.min(unread.len());
                    login_buffer.extend_from_slice(&unread[..take]);
                    unread = &unread[take..];

                    if login_buffer.len() < LOGIN_BLOCK_SIZE {
                        continue;
                    }

                    let login = LoginBlock::parse(&login_buffer)?;
                    info!(%id, name = %login.name, client_version = ?login.client_version, "legacy login block parsed");
                    events.send(SessionEvent::Login { id, login }).await.ok();
                    logged_in = true;
                }

                decoder.push(unread);
                while let Some(command) = decoder.next_command()? {
                    debug!(%id, command = command.kind.0, len = command.bytes.len(), "legacy command received");
                    let command_kind = command.kind.0;
                    let action = ClientAction::try_from(&command)?;
                    if events
                        .send(SessionEvent::Action {
                            id,
                            command_kind,
                            action,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
            command = commands.recv() => {
                match command {
                    Some(SessionCommand::Send(payload)) => {
                        let frame = encode_tick_frame(&payload)?;
                        socket.write_all(&frame).await?;
                    }
                    Some(SessionCommand::Disconnect) | None => break,
                }
            }
        }
    }

    events.send(SessionEvent::Disconnected { id }).await.ok();
    info!(%id, %peer_addr, "client disconnected");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    use tokio::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
        sync::mpsc,
    };
    use ugaris_protocol::login::{
        decrypt_password, LOGIN_BLOCK_SIZE, PASSWORD_SIZE, UGARIS_VENDOR_PREFIX,
    };

    use super::*;

    #[tokio::test]
    async fn session_accepts_legacy_login_block() {
        let listener = TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (command_tx, command_rx) = mpsc::channel(8);

        let server_task = tokio::spawn(async move {
            let (socket, peer_addr) = listener.accept().await.unwrap();
            run_session(
                SessionId(1),
                socket,
                peer_addr,
                events_tx,
                command_tx,
                command_rx,
            )
            .await
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        let login_block = legacy_login_block("Tester", "secret", 3);
        client.write_all(&login_block).await.unwrap();

        let connected_event = events_rx.recv().await.unwrap();
        let _commands = match connected_event {
            SessionEvent::Connected { commands, .. } => commands,
            _ => panic!("expected connected event"),
        };
        let mut login = None;
        while let Some(event) = events_rx.recv().await {
            match event {
                SessionEvent::Login { login: parsed, .. } => {
                    login = Some(parsed);
                    break;
                }
                SessionEvent::Disconnected { .. } => break,
                _ => {}
            }
        }
        let login = login.expect("expected login event before disconnect");
        assert_eq!(login.name, "Tester");
        assert_eq!(login.password, "secret");
        assert_eq!(login.client_version, Some(3));
        server_task.abort();
    }

    fn legacy_login_block(
        name: &str,
        password: &str,
        client_version: u8,
    ) -> [u8; LOGIN_BLOCK_SIZE] {
        let mut block = [0_u8; LOGIN_BLOCK_SIZE];
        let name_len = name.len().min(40);
        block[..name_len].copy_from_slice(&name.as_bytes()[..name_len]);

        let mut encrypted_password = [0_u8; PASSWORD_SIZE];
        let password_len = password.len().min(PASSWORD_SIZE);
        encrypted_password[..password_len].copy_from_slice(&password.as_bytes()[..password_len]);
        decrypt_password(&block[..40], &mut encrypted_password);
        block[40..56].copy_from_slice(&encrypted_password);
        block[56..60]
            .copy_from_slice(&(UGARIS_VENDOR_PREFIX | u32::from(client_version)).to_le_bytes());
        block
    }
}
