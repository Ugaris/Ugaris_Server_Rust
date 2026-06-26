use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use tokio::{net::TcpListener, sync::mpsc};
use tracing::{info, warn};

use crate::session::{run_session, SessionCommand, SessionEvent, SessionId};

#[derive(Debug)]
pub struct NetServer {
    bind_addr: SocketAddr,
    next_session: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerStatus {
    pub bind_addr: SocketAddr,
    pub listener_count: usize,
}

impl NetServer {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            next_session: AtomicU64::new(1),
        }
    }

    pub async fn run(
        self,
        events: mpsc::Sender<SessionEvent>,
        ready: Option<tokio::sync::oneshot::Sender<Result<ListenerStatus, String>>>,
    ) -> anyhow::Result<()> {
        let listener = match TcpListener::bind(self.bind_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                if let Some(ready) = ready {
                    let _ = ready.send(Err(err.to_string()));
                }
                return Err(err.into());
            }
        };
        let primary_addr = listener.local_addr()?;
        info!(addr = %primary_addr, "listening for legacy clients");

        let next_session = Arc::new(self.next_session);
        let mut listener_count = 1;
        if self.bind_addr.is_ipv4() && self.bind_addr.ip().is_unspecified() {
            match bind_ipv6_localhost(primary_addr.port()).await {
                Ok(ipv6_listener) => {
                    listener_count += 1;
                    spawn_accept_loop(ipv6_listener, events.clone(), next_session.clone());
                    info!(addr = %SocketAddrV6::new(Ipv6Addr::LOCALHOST, primary_addr.port(), 0, 0), "listening for IPv6 localhost legacy clients");
                }
                Err(err) => {
                    warn!(error = %err, port = primary_addr.port(), "could not bind IPv6 localhost listener; IPv4 listener remains active");
                }
            }
        }

        spawn_accept_loop(listener, events, next_session);
        info!(listener_count, "legacy TCP listener setup complete");
        if let Some(ready) = ready {
            let _ = ready.send(Ok(ListenerStatus {
                bind_addr: primary_addr,
                listener_count,
            }));
        }

        std::future::pending::<()>().await;
        Ok(())
    }
}

async fn bind_ipv6_localhost(port: u16) -> anyhow::Result<TcpListener> {
    Ok(TcpListener::bind(SocketAddr::V6(SocketAddrV6::new(
        Ipv6Addr::LOCALHOST,
        port,
        0,
        0,
    )))
    .await?)
}

fn spawn_accept_loop(
    listener: TcpListener,
    events: mpsc::Sender<SessionEvent>,
    next_session: Arc<AtomicU64>,
) {
    tokio::spawn(async move {
        loop {
            let (socket, peer_addr) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(err) => {
                    warn!(error = %err, "legacy TCP accept failed");
                    continue;
                }
            };
            let session_id = SessionId(next_session.fetch_add(1, Ordering::Relaxed));
            let events = events.clone();
            let (command_tx, command_rx) = mpsc::channel::<SessionCommand>(512);
            tokio::spawn(async move {
                if let Err(err) = run_session(
                    session_id, socket, peer_addr, events, command_tx, command_rx,
                )
                .await
                {
                    warn!(%session_id, %peer_addr, error = %err, "session ended with error");
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use tokio::{net::TcpStream, sync::oneshot};

    use super::*;

    #[tokio::test]
    async fn server_reports_ready_and_accepts_loopback_connection() {
        let bind_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (ready_tx, ready_rx) = oneshot::channel();
        let server = NetServer::new(bind_addr);

        let handle = tokio::spawn(server.run(events_tx, Some(ready_tx)));
        let status = ready_rx.await.unwrap().unwrap();
        let _client = TcpStream::connect(status.bind_addr).await.unwrap();

        let event = events_rx.recv().await.unwrap();
        assert!(matches!(event, SessionEvent::Connected { .. }));
        handle.abort();
    }

    #[tokio::test]
    async fn unspecified_ipv4_bind_also_accepts_localhost_ipv6() {
        let bind_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
        let (events_tx, mut events_rx) = mpsc::channel(8);
        let (ready_tx, ready_rx) = oneshot::channel();
        let server = NetServer::new(bind_addr);

        let handle = tokio::spawn(server.run(events_tx, Some(ready_tx)));
        let status = ready_rx.await.unwrap().unwrap();
        let ipv6_localhost = SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::LOCALHOST,
            status.bind_addr.port(),
            0,
            0,
        ));
        let _client = TcpStream::connect(ipv6_localhost).await.unwrap();

        let event = events_rx.recv().await.unwrap();
        assert!(matches!(event, SessionEvent::Connected { .. }));
        assert!(status.listener_count >= 1);
        handle.abort();
    }
}
