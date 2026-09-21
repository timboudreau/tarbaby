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
use crate::{headers::random_header_line, stats::STATS, stats_logger::StatsLogger};
use chrono::{DateTime, Utc};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt as _};

const GENERIC_RESPONSE_HEAD: &[u8] = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n";

#[derive(Clone)]
pub struct RequestHandlerFactory {
    pub millis_between_headers: u64,
    pub stats_logger: Arc<StatsLogger>,
    pub max_headers_to_send: usize,
    pub startup: Instant,
}

impl RequestHandlerFactory {
    pub fn new_handler<S: AsyncWrite + AsyncReadExt + Unpin>(
        &self,
        socket: S,
    ) -> RequestHandler<S> {
        RequestHandler {
            socket,
            delay: self.millis_between_headers,
            stats_logger: self.stats_logger.clone(),
            max_headers_to_send: self.max_headers_to_send,
            startup: self.startup,
        }
    }
}

pub struct RequestHandler<S: AsyncWrite + Unpin> {
    pub socket: S,
    pub delay: u64,
    pub stats_logger: Arc<StatsLogger>,
    pub max_headers_to_send: usize,
    pub startup: Instant,
}

impl<S: AsyncWrite + Unpin + AsyncReadExt> RequestHandler<S> {
    pub async fn handle_request(
        mut self,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        STATS.on_request();
        let addr_string = addr.to_string();

        // We want enough to log the request for forensic purposes, no more.  We aren't actually
        // interested in what was requested.
        let mut buffer = [0; 128];
        let n = self.socket.read(&mut buffer).await?;
        if n == 0 {
            // nothing to read; bail.
            return Ok(());
        }
        let request = String::from_utf8_lossy(&buffer[..n]).to_string();

        tracing::info!(address = addr_string, request = request, "request");

        match self.socket.write_all(GENERIC_RESPONSE_HEAD).await {
            Ok(_) => {
                tracing::trace!(address = addr_string, "Response line sent");
                if let Err(e) = self.socket.flush().await {
                    return Err(Box::new(e));
                }
            }
            Err(e) => return Err(Box::new(e)),
        }

        let date_header = Self::date_header();
        // Do *some* initial delay, but not the full one, before the date header.
        tokio::time::sleep(Duration::from_millis(self.delay / 4)).await;
        self.socket
            .write_all(date_header.as_bytes())
            .await
            .map_err(Box::new)?;

        // Start sending headers until the end of time:
        self.headers_loop(GENERIC_RESPONSE_HEAD.len() + date_header.len(), addr_string)
            .await
    }

    fn date_header() -> String {
        format!(
            "Date: {}\r\n",
            DateTime::<Utc>::from(SystemTime::now())
                .to_rfc2822()
                .replace(" +0000", " GMT")
        )
    }

    async fn headers_loop(
        mut self,
        mut bytes: usize,
        addr_string: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut bogus_header_count = 0_usize;
        let mut headers = Vec::with_capacity(48);
        loop {
            tokio::time::sleep(self.next_delay()).await;
            random_header_line(&mut headers);
            bytes += headers.len();
            match self.socket.write_all(&headers).await {
                Ok(_) => {
                    headers.clear();
                    bogus_header_count += 1;
                    STATS.on_header(bogus_header_count);
                    if bogus_header_count.is_multiple_of(100) {
                        tracing::info!(
                            address = addr_string,
                            bytes = bytes,
                            "{} sent and still going :-)",
                            bogus_header_count
                        )
                    }
                    self.stats_logger.maybe_log();
                    if bogus_header_count == self.max_headers_to_send {
                        tracing::info!(
                            address = addr_string,
                            "reached max header limit {}. Killing connection.",
                            self.max_headers_to_send
                        );
                        return Ok(());
                    }
                }
                Err(e) => {
                    tracing::info!(
                        headers_sent = bogus_header_count,
                        address = addr_string,
                        bytes = bytes,
                        "Client gave up: {}",
                        e
                    );
                    break;
                }
            }
        }
        self.stats_logger.maybe_log();
        Ok(())
    }

    fn next_delay(&self) -> Duration {
        // Include an eighth of a second's jitter to keep things from being bursty
        // if two timers happen to land on the same subsecond interval
        Duration::from_millis(self.delay) + self.jitter::<12500000>()
    }

    fn jitter<const MAX: u64>(&self) -> Duration {
        Duration::from_nanos((self.startup.elapsed().as_nanos() % (MAX as u128 + 1)) as u64)
    }
}
