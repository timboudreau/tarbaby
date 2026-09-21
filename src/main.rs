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
use crate::{cli::Command, server::TarbabyServer};
use clap::Parser;
use std::process::exit;

mod cli;
mod headers;
mod logging;
mod request_handler;
mod server;
mod stats;
mod stats_logger;

#[cfg(not(feature = "multithread"))]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // init logging as early as possible
    common_main().await
}

#[cfg(feature = "multithread")]
#[tokio::main]
async fn main() {
    // init logging as early as possible
    common_main().await
}

async fn common_main() {
    match Command::parse() {
        Command::Serve {
            port,
            interface,
            millis_between_headers,
            minutes_between_logging_stats,
            max_headers_to_send,
            certificate,
            key,
            log_dir,
            async_logging,
            json_logging,
        } => {
            let logger_guard = crate::logging::init_logging(log_dir, async_logging, json_logging);
            if certificate.is_some() != key.is_some() {
                tracing::error!(
                    "Need both cert and key, got {:?} and {:?}",
                    certificate,
                    key
                );
                eprintln!("Either both --key and --certificate must be provided, or neither.");
                exit(1);
            }
            let cert_and_key = if let (Some(certificate), Some(key)) = (certificate, key) {
                Some((certificate, key))
            } else {
                None
            };

            TarbabyServer::new(
                millis_between_headers.max(1),
                max_headers_to_send,
                minutes_between_logging_stats,
                logger_guard,
            )
            .serve(port, interface, cert_and_key)
            .await
            .expect("Failed to start")
        }
    }
}
