use std::io::Result;
use std::sync::Arc;

use log::{info, warn};
use tokio::net::TcpListener;

use crate::config::base::{InboundConfig, OutboundConfig};
use crate::protocol::common::stream::InboundStream;
use crate::proxy::acceptor::Acceptor;
use crate::proxy::base::SupportedProtocols;
use crate::proxy::handler::Handler;

pub struct TcpServer {
    local_addr_port: (String, u16),
    protocol: SupportedProtocols,
    secret: Option<String>,
    acceptor: Arc<Acceptor>,
    handler: Arc<Handler>,
}

impl TcpServer {
    pub fn new(
        inbound_config: InboundConfig,
        outbound_config: OutboundConfig,
    ) -> Result<TcpServer> {
        let handler = Arc::from(Handler::new(&outbound_config)?);
        let acceptor = Arc::from(Acceptor::new(&inbound_config));

        let secret = match inbound_config.secret {
            Some(secret) => Some(secret),
            None => None,
        };

        return Ok(TcpServer {
            local_addr_port: (inbound_config.address, inbound_config.port),
            protocol: inbound_config.protocol,
            secret,
            handler,
            acceptor,
        });
    }

    pub async fn start(self) -> Result<()> {
        let (local_addr, local_port) = self.local_addr_port;

        let listener = TcpListener::bind(format!("{}:{}", local_addr, local_port)).await?;

        let acceptor = self.acceptor;

        info!(
            "TCP server started on {}:{}, ready to accept input stream",
            local_addr, local_port
        );

        loop {
            let (socket, addr) = listener.accept().await?;

            info!("Received new connection from {}", addr);

            let acceptor_clone = Arc::clone(&acceptor);
            let handler = Arc::clone(&self.handler);

            tokio::spawn(async move {
                let mut inbound_stream = match acceptor_clone.accept(socket).await {
                    Ok(stream) => stream,
                    Err(e) => {
                        warn!("Failed to accept inbound connection from {}: {}", addr, e);
                        return;
                    }
                };

                match handler.dispatch(&mut inbound_stream).await {
                    Ok(_) => {
                        info!("Connection to {} has finished", addr);
                    }
                    Err(e) => {
                        warn!("Failed to handle the inbound stream: {}", e);
                    }
                }
            });
        }
    }
}
