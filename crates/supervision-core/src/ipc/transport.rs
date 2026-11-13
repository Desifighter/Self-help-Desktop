use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn, error};
use super::protocol::{IpcRequest, IpcResponse};

pub const IPC_PORT: u16 = 49215;
pub const IPC_ADDR: &str = "127.0.0.1:49215";

pub struct IpcClient;

impl IpcClient {
    pub async fn send_request(req: &IpcRequest) -> io::Result<IpcResponse> {
        let mut stream = TcpStream::connect(IPC_ADDR).await?;
        let payload = serde_json::to_vec(req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let len_bytes = (payload.len() as u32).to_be_bytes();
        stream.write_all(&len_bytes).await?;
        stream.write_all(&payload).await?;

        // Read response length
        let mut resp_len_buf = [0u8; 4];
        stream.read_exact(&mut resp_len_buf).await?;
        let resp_len = u32::from_be_bytes(resp_len_buf) as usize;

        let mut resp_buf = vec![0u8; resp_len];
        stream.read_exact(&mut resp_buf).await?;

        let resp: IpcResponse = serde_json::from_slice(&resp_buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(resp)
    }
}
