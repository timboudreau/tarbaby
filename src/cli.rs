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
use std::path::PathBuf;

/// Defines the clap command-line arguments and interface.
#[derive(Clone, clap::Parser)]
#[command(name = "tarbaby")]
#[command(version = "1.0.0")]
#[command(about = "A reverse slowloris attack for vulnerability scanners.", long_about = None)]
pub enum Command {
    /// Start the tarbaby server
    Serve {
        /// The port to run on.
        #[arg(short, long, default_value = "6666")]
        port: u16,

        /// The network address to bind to.  The default binds all IPv4 and IPv6 addresses on
        /// all interfaces.
        #[arg(short, long, default_value = "::0")]
        interface: String,

        /// The amount of time to pause after each bogus header.
        #[arg(short, long, default_value = "890")]
        millis_between_headers: u64,

        /// The interval, in minutes, of how frequently we should log
        /// activity stats (needs an active request).
        #[arg(short = 'b', long, default_value = "60")]
        minutes_between_logging_stats: u64,

        /// Max headers to send after which the connection will be killed, if you're
        /// feeling charitable.  The default is `usize::MAX` which is effectively, never.
        #[arg(short = 'x', long, default_value = "18446744073709551615")]
        max_headers_to_send: usize,

        /// Path to a certificate (.pem) file, for serving HTTPS.  If this is
        /// set, `key_path` must also be passed.
        #[arg(short, long)]
        certificate: Option<PathBuf>,

        /// Path to a key file (.pem) to go with the certificate file, for serving HTTPS.  If this is
        /// set, `certificate` must also be passed.
        #[arg(short, long)]
        key: Option<PathBuf>,

        /// If set, log to daily rotated files in this directory, instead of to the console.
        #[arg(short, long)]
        log_dir: Option<PathBuf>,

        /// If set, use asynchronous logging so the request/response cycle cannot block on logging I/O.
        #[arg(short, long)]
        async_logging: bool,

        /// If set, log records in jsonl one-line-per-record format instead of the more human-friendly
        /// version
        #[arg(short, long = "json")]
        json_logging: bool,
    },
}
