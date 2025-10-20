use anyhow::{Result, anyhow};
use crate::types::{PluginMessage, PluginConfig};
use tokio::net::{UnixStream, UnixListener, TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;
use bincode;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};

/// IPC transport abstraction
#[derive(Debug)]
pub enum Transport {
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl Transport {
    /// Send a message over the transport
    async fn send(&mut self, msg: &PluginMessage) -> Result<()> {
        let data = bincode::serialize(msg)?;
        let len = data.len() as u32;
        let len_bytes = len.to_be_bytes();

        match self {
            Transport::Unix(stream) => {
                stream.write_all(&len_bytes).await?;
                stream.write_all(&data).await?;
                stream.flush().await?;
            }
            Transport::Tcp(stream) => {
                stream.write_all(&len_bytes).await?;
                stream.write_all(&data).await?;
                stream.flush().await?;
            }
        }
        Ok(())
    }

    /// Receive a message from the transport
    async fn recv(&mut self) -> Result<PluginMessage> {
        let mut len_bytes = [0u8; 4];

        match self {
            Transport::Unix(stream) => {
                stream.read_exact(&mut len_bytes).await?;
            }
            Transport::Tcp(stream) => {
                stream.read_exact(&mut len_bytes).await?;
            }
        }

        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > 1024 * 1024 * 10 { // 10MB max message size
            return Err(anyhow!("Message too large: {} bytes", len));
        }

        let mut data = vec![0u8; len];
        match self {
            Transport::Unix(stream) => {
                stream.read_exact(&mut data).await?;
            }
            Transport::Tcp(stream) => {
                stream.read_exact(&mut data).await?;
            }
        }

        let msg = bincode::deserialize(&data)?;
        Ok(msg)
    }
}

/// IPC client for plugins to connect to host
pub struct IPCClient {
    transport: Arc<Mutex<Transport>>,
    rx: mpsc::UnboundedReceiver<PluginMessage>,
    _handle: tokio::task::JoinHandle<()>,
}

impl IPCClient {
    /// Connect to a Unix socket
    pub async fn connect_unix<P: AsRef<Path>>(path: P) -> Result<Self> {
        let stream = UnixStream::connect(path).await?;
        Self::new(Transport::Unix(stream)).await
    }

    /// Connect to a TCP socket
    pub async fn connect_tcp(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Self::new(Transport::Tcp(stream)).await
    }

    /// Create client from config
    pub async fn from_config(config: &PluginConfig) -> Result<Self> {
        if let Some(socket_path) = &config.socket_path {
            Self::connect_unix(socket_path).await
        } else if let Some(tcp_addr) = &config.tcp_addr {
            Self::connect_tcp(tcp_addr).await
        } else {
            Err(anyhow!("No connection configuration provided"))
        }
    }

    async fn new(mut transport: Transport) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let transport = Arc::new(Mutex::new(transport));
        let transport_clone = transport.clone();

        // Spawn reader task
        let handle = tokio::spawn(async move {
            loop {
                let msg = {
                    let mut t = transport_clone.lock().await;
                    t.recv().await
                };

                match msg {
                    Ok(msg) => {
                        if tx.send(msg).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            transport,
            rx,
            _handle: handle,
        })
    }

    /// Send a message
    pub async fn send(&self, msg: PluginMessage) -> Result<()> {
        let mut transport = self.transport.lock().await;
        transport.send(&msg).await
    }

    /// Try to receive a message
    pub fn try_recv(&mut self) -> Option<PluginMessage> {
        self.rx.try_recv().ok()
    }

    /// Receive a message (blocking)
    pub async fn recv(&mut self) -> Option<PluginMessage> {
        self.rx.recv().await
    }

    /// Send and wait for response
    pub async fn request(&self, msg: PluginMessage, timeout_ms: u64) -> Result<PluginMessage> {
        self.send(msg).await?;

        let duration = Duration::from_millis(timeout_ms);
        match timeout(duration, self.transport.lock()).await {
            Ok(mut transport) => transport.recv().await,
            Err(_) => Err(anyhow!("Request timed out")),
        }
    }
}

/// IPC server for host applications
pub struct IPCServer {
    listener: IPCListener,
}

enum IPCListener {
    Unix(UnixListener),
    Tcp(TcpListener),
}

impl IPCServer {
    /// Create a Unix socket server
    pub async fn bind_unix<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Remove existing socket file if it exists
        if path.as_ref().exists() {
            std::fs::remove_file(&path)?;
        }

        let listener = UnixListener::bind(path)?;
        Ok(Self {
            listener: IPCListener::Unix(listener),
        })
    }

    /// Create a TCP server
    pub async fn bind_tcp(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener: IPCListener::Tcp(listener),
        })
    }

    /// Accept a new connection
    pub async fn accept(&self) -> Result<IPCConnection> {
        match &self.listener {
            IPCListener::Unix(listener) => {
                let (stream, _) = listener.accept().await?;
                Ok(IPCConnection::new(Transport::Unix(stream)).await)
            }
            IPCListener::Tcp(listener) => {
                let (stream, _) = listener.accept().await?;
                Ok(IPCConnection::new(Transport::Tcp(stream)).await)
            }
        }
    }
}

/// A single IPC connection
pub struct IPCConnection {
    pub id: uuid::Uuid,
    transport: Arc<Mutex<Transport>>,
    pub rx: mpsc::UnboundedReceiver<PluginMessage>,
    _handle: tokio::task::JoinHandle<()>,
}

impl IPCConnection {
    async fn new(transport: Transport) -> Self {
        let id = uuid::Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel();
        let transport = Arc::new(Mutex::new(transport));
        let transport_clone = transport.clone();

        // Spawn reader task
        let handle = tokio::spawn(async move {
            loop {
                let msg = {
                    let mut t = transport_clone.lock().await;
                    t.recv().await
                };

                match msg {
                    Ok(msg) => {
                        if tx.send(msg).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            id,
            transport,
            rx,
            _handle: handle,
        }
    }

    /// Send a message
    pub async fn send(&self, msg: PluginMessage) -> Result<()> {
        let mut transport = self.transport.lock().await;
        transport.send(&msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PluginMessage;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_unix_socket_communication() {
        let socket_path = "/tmp/test_ratacat_plugin.sock";

        // Start server
        let server = IPCServer::bind_unix(socket_path).await.unwrap();

        // Server task
        let server_handle = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();

            if let Some(msg) = conn.rx.recv().await {
                match msg {
                    PluginMessage::Ping { timestamp } => {
                        conn.send(PluginMessage::Pong { timestamp }).await.unwrap();
                    }
                    _ => {}
                }
            }
        });

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        // Connect client
        let mut client = IPCClient::connect_unix(socket_path).await.unwrap();

        // Send ping
        let timestamp = chrono::Utc::now();
        client.send(PluginMessage::Ping { timestamp }).await.unwrap();

        // Receive pong
        let response = client.recv().await.unwrap();
        match response {
            PluginMessage::Pong { timestamp: pong_time } => {
                assert_eq!(timestamp, pong_time);
            }
            _ => panic!("Expected Pong message"),
        }

        server_handle.abort();

        // Cleanup
        std::fs::remove_file(socket_path).ok();
    }
}