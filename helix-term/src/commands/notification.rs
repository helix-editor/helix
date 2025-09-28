use crate::compositor::Context;
use crate::job::{self, Callback};
use helix_view::editor::Severity;

/// Show notification history
pub fn show_notification_history(cx: &mut Context) {
    let history = cx.editor.get_notification_history();
    
    if history.is_empty() {
        cx.editor.set_status("No notifications in history");
        return;
    }

    let mut content = String::new();
    content.push_str("Notification History:\n\n");
    
    for (i, notification) in history.iter().enumerate().rev().take(50) {
        let severity_icon = match notification.severity {
            Severity::Error => "❌",
            Severity::Warning => "⚠️",
            Severity::Info => "ℹ️",
            Severity::Hint => "💡",
        };
        
        let timestamp = notification.timestamp.elapsed().as_secs();
        let time_str = if timestamp < 60 {
            format!("{}s ago", timestamp)
        } else if timestamp < 3600 {
            format!("{}m ago", timestamp / 60)
        } else {
            format!("{}h ago", timestamp / 3600)
        };
        
        content.push_str(&format!(
            "{:2}. {} {} ({})\n    {}\n\n",
            history.len() - i,
            severity_icon,
            time_str,
            if notification.dismissed { "dismissed" } else { "active" },
            notification.message
        ));
    }

    let popup = crate::ui::Popup::new("notification-history", crate::ui::Text::new(content))
        .auto_close(true);
    
    cx.jobs.callback(async move {
        let call: job::Callback = Callback::EditorCompositor(Box::new(move |_editor, compositor| {
            compositor.push(Box::new(popup));
        }));
        Ok(call)
    });
}

/// Clear notification history
pub fn clear_notification_history(cx: &mut Context) {
    cx.editor.clear_notification_history();
    cx.editor.set_status("Notification history cleared");
}

/// Dismiss all active notifications
pub fn dismiss_all_notifications(cx: &mut Context) {
    cx.editor.dismiss_all_notifications();
    cx.editor.set_status("All notifications dismissed");
}

/// Test notification system with sample notifications
pub fn test_notifications(cx: &mut Context) {
    let config = &cx.editor.config().notifications;
    let timeout_ms = config.default_timeout.as_millis();
    
    // Debug output to log
    log::warn!("DEBUG: Creating notification with timeout: {:?} ({}ms)", config.default_timeout, timeout_ms);
    
    // Create a simple test notification
    let id = cx.editor.notify_info(format!("Test notification (timeout: {}ms) - should disappear in {}s", 
                                          timeout_ms, timeout_ms as f64 / 1000.0));
    
    // Check if the notification was created with timeout
    let all_notifications = cx.editor.get_notification_history();
    if let Some(notification) = all_notifications.last() {
        log::warn!("DEBUG: Notification {} created with timeout: {:?}", notification.id, notification.timeout);
    }
    
    cx.editor.set_status(format!("Test notification sent with {}ms timeout - check terminal for debug", timeout_ms));
}
