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
use crate::stats::STATS;
use chrono::{DateTime, Local};
use std::{
    sync::atomic::{
        AtomicU64,
        Ordering::{Acquire, Release},
    },
    time::{Duration, SystemTime},
};

/// A thing that updates a deadline for the next time it should log stats and logs them
/// if the deadline has passed when `maybe_log()` is called.
pub struct StatsLogger {
    interval: Duration,
    next_deadline: AtomicU64,
}

impl StatsLogger {
    pub fn new(interval: Duration) -> Self {
        let initial_deadline = SystemTime::now() + interval;
        tracing::debug!(
            every = chrono::Duration::from_std(interval)
                .expect("Could not convert_duration")
                .to_string(),
            first_deadline = Self::format_date(initial_deadline),
            "init stats-logger"
        );
        Self {
            interval,
            next_deadline: AtomicU64::new(Self::epoch_millis(initial_deadline)),
        }
    }

    /// Log the stats if we are past the deadline.
    pub fn maybe_log(&self) {
        if let Some((period_start, period_end)) = self.touch() {
            STATS.snapshot().log(period_start, period_end);
        }
    }

    fn touch(&self) -> Option<(SystemTime, SystemTime)> {
        let now_time = SystemTime::now();
        let now = Self::epoch_millis(now_time);
        let result = self.next_deadline.try_update(Release, Acquire, |old| {
            if old < now {
                Some(now + self.interval.as_millis() as u64)
            } else {
                None
            }
        });
        if let Ok(old_deadline) = result {
            let old_start =
                (SystemTime::UNIX_EPOCH + Duration::from_millis(old_deadline)) - self.interval;
            tracing::debug!(
                deadline = Self::format_date(now_time + self.interval),
                "next-stats"
            );
            Some((old_start, now_time))
        } else {
            None
        }
    }

    fn epoch_millis(when: SystemTime) -> u64 {
        when.duration_since(SystemTime::UNIX_EPOCH)
            .expect("Cannot compute system time millis")
            .as_millis() as u64
    }

    fn format_date(when: SystemTime) -> String {
        let loc = DateTime::<Local>::from(when);
        loc.to_rfc2822()
    }
}
