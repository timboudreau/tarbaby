/*
Copyright (C) 2026 Tim Boudreau

This program is free software; you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation; either version 2 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program; if not, write to the Free Software
Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/
use crate::{
    logging::LoggerGuard, request_handler::RequestHandlerFactory, stats::STATS,
    stats_logger::StatsLogger,
};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, rustls::ServerConfig};

pub struct TarbabyServer {
    /// Factory for individual request handlers
    request_handlers: RequestHandlerFactory,

    /// If async logging is enabled, this needs to hold a reference to the
    /// logger impl, or it will be dropped and the background logging thread will exit.
    #[allow(unused)]
    logger_guard: LoggerGuard,
}

impl TarbabyServer {
    pub fn new(
        millis_between_headers: u64,
        max_headers_to_send: usize,
        minutes_between_logging_stats: u64,
        logger_guard: LoggerGuard,
    ) -> Self {
        Self {
            request_handlers: RequestHandlerFactory {
                millis_between_headers,
                stats_logger: Arc::new(StatsLogger::new(Duration::from_mins(
                    minutes_between_logging_stats.max(1),
                ))),
                max_headers_to_send,
                startup: Instant::now(),
            },
            logger_guard,
        }
    }

    pub async fn serve(
        self,
        port: u16,
        interface: String,
        cert_and_key: Option<(PathBuf, PathBuf)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dur = chrono::Duration::from_std(Duration::from_millis(
            self.request_handlers.millis_between_headers,
        ))
        .expect("Could not convert duration");

        tracing::info!(
            interface = interface,
            port = port,
            time_between_headers = dur.to_string(),
            "Starting server"
        );

        let listener = TcpListener::bind(format!("{}:{}", interface, port)).await?;
        tracing::trace!("Server socket opened: {:?}", listener);
        if let Some((cert, key)) = cert_and_key {
            self.serve_https(listener, cert, key).await
        } else {
            self.serve_plain(listener).await
        }
    }

    async fn serve_https(
        self,
        listener: TcpListener,
        cert: PathBuf,
        key: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cert = Self::load_certs(&cert)?;
        let key = Self::load_key(&key)?;

        let mut tls_server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert, key)?;

        tls_server_config.alpn_protocols = vec![b"http/1.1".to_vec()];

        let acceptor = TlsAcceptor::from(Arc::new(tls_server_config));

        tracing::info!("TLS configured");
        loop {
            let (stream, addr) = listener.accept().await?;
            tracing::trace!("Connect from {}", addr);

            let acceptor = acceptor.clone();
            let handlers = self.request_handlers.clone();
            tokio::spawn(async move {
                // Perform the cryptographic handshake
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        let handler = handlers.new_handler(tls_stream);
                        if let Err(e) = handler.handle_request(addr).await {
                            tracing::error!("Failed to handle connection: {}", e);
                        }
                    }
                    Err(e) => tracing::error!("TLS handshake error: {}", e),
                }
                STATS.on_close();
            });
        }
    }

    async fn serve_plain(self, listener: TcpListener) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let (socket, addr) = listener.accept().await?;
            tracing::trace!("Connect from {}", addr);

            let handler = self.request_handlers.new_handler(socket);
            tokio::spawn(async move {
                if let Err(e) = handler.handle_request(addr).await {
                    tracing::error!("Failed to handle connection: {}", e);
                }
                STATS.on_close();
            });
        }
    }

    fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, std::io::Error> {
        let certfile = File::open(path)?;
        let mut reader = BufReader::new(certfile);

        rustls_pemfile::certs(&mut reader).collect()
    }

    // Helper function to read the private key into Memory
    fn load_key(path: &Path) -> Result<PrivateKeyDer<'static>, std::io::Error> {
        let keyfile = File::open(path)?;
        let mut reader = BufReader::new(keyfile);
        rustls_pemfile::private_key(&mut reader)?.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Private key not found")
        })
    }
}
