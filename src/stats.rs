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
use std::{
    sync::atomic::{
        AtomicUsize,
        Ordering::{Relaxed, SeqCst},
    },
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Local};

/// Global stats the server updates.
pub static STATS: Stats = Stats::new();

/// Keeps count of what the server has done so we can periodically log it.
pub struct Stats {
    requests: AtomicUsize,
    headers_sent: AtomicUsize,
    max_headers_sent_to_one_client: AtomicUsize,
    connections_closed: AtomicUsize,
    // Stats we don't clear on snapshot
    cumulative_requests: AtomicUsize,
    cumulative_headers_sent: AtomicUsize,
    max_headers_ever_sent_to_one_client: AtomicUsize,
    cumulative_connections_closed: AtomicUsize,
}

impl Stats {
    const fn new() -> Self {
        Self {
            requests: AtomicUsize::new(0),
            headers_sent: AtomicUsize::new(0),
            max_headers_sent_to_one_client: AtomicUsize::new(0),
            connections_closed: AtomicUsize::new(0),
            // cumulative stats
            cumulative_requests: AtomicUsize::new(0),
            cumulative_headers_sent: AtomicUsize::new(0),
            max_headers_ever_sent_to_one_client: AtomicUsize::new(0),
            cumulative_connections_closed: AtomicUsize::new(0),
        }
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            // Swap the period-specific stats with 0's, read the rest
            requests: self.requests.swap(0, SeqCst),
            headers_sent_this_period: self.headers_sent.swap(0, SeqCst),
            max_headers_sent_to_one_client_this_period: self
                .max_headers_sent_to_one_client
                .swap(0, SeqCst),
            connections_closed_this_period: self.connections_closed.swap(0, SeqCst),
            cumulative_requests: self.cumulative_requests.load(SeqCst),
            cumulative_headers_sent: self.cumulative_headers_sent.load(SeqCst),
            max_headers_ever_sent_to_one_client: self
                .max_headers_ever_sent_to_one_client
                .load(SeqCst),
            cumulative_connections_closed: self.cumulative_connections_closed.load(SeqCst),
        }
    }

    pub fn on_request(&self) {
        self.requests.fetch_add(1, Relaxed);
        self.cumulative_requests.fetch_add(1, Relaxed);
    }

    pub fn on_close(&self) {
        self.connections_closed.fetch_add(1, Relaxed);
        self.cumulative_connections_closed.fetch_add(1, Relaxed);
    }

    pub fn on_header(&self, sent_to_this_client: usize) {
        self.max_headers_sent_to_one_client
            .fetch_max(sent_to_this_client, Relaxed);
        self.headers_sent.fetch_add(1, Relaxed);
        self.cumulative_headers_sent.fetch_add(1, Relaxed);
        self.max_headers_ever_sent_to_one_client
            .fetch_max(sent_to_this_client, Relaxed);
    }
}

#[derive(Copy, Clone)]
pub struct StatsSnapshot {
    pub requests: usize,
    pub headers_sent_this_period: usize,
    pub max_headers_sent_to_one_client_this_period: usize,
    pub connections_closed_this_period: usize,
    pub cumulative_requests: usize,
    pub cumulative_headers_sent: usize,
    pub max_headers_ever_sent_to_one_client: usize,
    pub cumulative_connections_closed: usize,
}

impl StatsSnapshot {
    pub fn is_empty(&self) -> bool {
        self.requests == 0
            && self.headers_sent_this_period == 0
            && self.max_headers_sent_to_one_client_this_period == 0
    }

    pub fn log(self, period_start: SystemTime, period_end: SystemTime) {
        let period_length = chrono::Duration::from_std(
            period_end
                .duration_since(period_start)
                .unwrap_or(Duration::ZERO),
        )
        .unwrap_or(chrono::Duration::zero())
        .to_string();

        let period_start = DateTime::<Local>::from(period_start)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        let period_end =
            DateTime::<Local>::from(period_end).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        if !self.is_empty() {
            // Since we are using Relaxed for updating stats for throughput, it is *conceivable*
            // that connections closed can be temporarily > cumulative requests, so use saturating subtraction.
            let live_connections = self
                .cumulative_requests
                .saturating_sub(self.cumulative_connections_closed);

            tracing::error!(
                period_start = period_start,
                period_end = period_end,
                period_length = period_length,
                active_connections = live_connections,
                requests = self.requests,
                connections_closed_this_period = self.connections_closed_this_period,
                headers_sent_this_period = self.headers_sent_this_period,
                max_headers_sent_to_one_client_this_period =
                    self.max_headers_sent_to_one_client_this_period,
                cumulative_requests = self.cumulative_requests,
                cumulative_headers_sent = self.cumulative_headers_sent,
                max_headers_ever_sent_to_one_client = self.max_headers_ever_sent_to_one_client,
                cumulative_connections_closed = self.cumulative_connections_closed,
                "stats"
            )
        } else {
            tracing::trace!(
                period_start = period_start,
                period_end = period_end,
                period_length = period_length.to_string(),
                "Empty stats; no log"
            );
        }
    }
}
