use helix_core::diagnostic::Severity;
use std::{cmp, time::Duration, time::Instant};

#[derive(Debug)]
pub struct Notification {
    /// Title shown at top.
    pub title: String,
    /// Text body, should contain newlines.
    pub text: String,
    /// Body height.
    pub height: u16,
    pub severity: Severity,
    pub created_at: Instant,
    pub timeout: Option<Instant>,
}

impl Notification {
    pub fn new(
        title: &str,
        text: String,
        severity: Severity,
        timeout: Option<Duration>,
    ) -> Notification {
        let mut height = text.split('\n').count();
        height = cmp::min(height, cmp::max(height, 8));
        let now = Instant::now();
        Notification {
            title: title.to_string(),
            text,
            height: height as u16,
            severity,
            created_at: now,
            timeout: now.checked_add(timeout.unwrap_or_else(|| Duration::from_millis(5000))),
        }
    }
}

#[derive(Debug)]
pub struct Notifications {
    /// TODO: settings?
    pub notifications: Vec<Notification>,
}

impl Notifications {
    pub fn new() -> Notifications {
        Notifications {
            notifications: Vec::new(),
        }
    }

    pub fn add(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    pub fn to_display(&mut self) -> Vec<&Notification> {
        let now = Instant::now();
        self.notifications
            .iter()
            .filter(|n| match n.timeout {
                Some(t) => t > now,
                None => false,
            })
            .collect()
    }
}

impl Default for Notifications {
    fn default() -> Self {
        Self::new()
    }
}
