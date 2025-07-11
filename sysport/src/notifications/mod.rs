#[derive(Debug, Clone)]
pub struct Notification {
    pub title: String,
    pub message: String,
}

pub struct NotificationManager;

impl NotificationManager {
    pub fn send_notification(notification: &Notification) {
        #[cfg(target_os = "linux")]
        {
            use notify_rust::Notification as NRNotification;
            let _ = NRNotification::new()
                .summary(&notification.title)
                .body(&notification.message)
                .show();
        }
        #[cfg(target_os = "macos")]
        {
            // TODO: Use mac-notification-sys or AppleScript
            println!("macOS notification: {} - {}", notification.title, notification.message);
        }
        #[cfg(target_os = "windows")]
        {
            // TODO: Use native Windows notification
            println!("Windows notification: {} - {}", notification.title, notification.message);
        }
    }
} 