#[cfg(feature = "dbus")]
pub mod dbus;
#[cfg(feature = "telegram")]
pub mod telegram;

use crate::config::Config;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

/// Trait defining a pluggable delivery mechanism for notifications.
pub trait Notifier: Send + Sync {
    /// Human-friendly name of the notification backend (e.g. "D-Bus", "Telegram")
    fn name(&self) -> &'static str;

    /// Dispatches the notification message asynchronously.
    fn send<'a>(
        &'a self,
        summary: &'a str,
        body: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

/// Dispatches notifications across one or more configured notification backends.
pub struct NotificationDispatcher {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotificationDispatcher {
    pub fn new() -> Self {
        Self {
            notifiers: Vec::new(),
        }
    }

    pub fn add<N: Notifier + 'static>(&mut self, notifier: N) {
        self.notifiers.push(Box::new(notifier));
    }

    pub fn is_empty(&self) -> bool {
        self.notifiers.is_empty()
    }

    /// Builds a dispatcher containing all active notifiers specified in the configuration.
    pub fn from_config(config: &Config) -> Self {
        let mut dispatcher = Self::new();

        for backend in &config.backends {
            match backend.trim().to_lowercase().as_str() {
                #[cfg(feature = "dbus")]
                "dbus" => {
                    dispatcher.add(dbus::DbusNotifier::new(&config.dbus_service));
                }
                #[cfg(not(feature = "dbus"))]
                "dbus" => {
                    eprintln!(
                        "Backend 'dbus' requested, but rmd was compiled without the 'dbus' feature."
                    );
                }

                #[cfg(feature = "telegram")]
                "telegram" => {
                    if let Some(ref tg_cfg) = config.telegram {
                        if let (Some(token), Some(chat_id)) =
                            (tg_cfg.get_token(), tg_cfg.get_chat_id())
                        {
                            dispatcher.add(telegram::TelegramNotifier::new(
                                token,
                                chat_id,
                                tg_cfg.get_endpoint(),
                            ));
                        } else {
                            eprintln!(
                                "Backend 'telegram' enabled, but bot_token or chat_id is missing."
                            );
                        }
                    } else if let (Ok(token), Ok(chat_id)) = (
                        std::env::var("RMD_TELEGRAM_BOT_TOKEN"),
                        std::env::var("RMD_TELEGRAM_CHAT_ID"),
                    ) {
                        let endpoint = std::env::var("RMD_TELEGRAM_ENDPOINT")
                            .unwrap_or_else(|_| "https://api.telegram.org".to_string());
                        dispatcher.add(telegram::TelegramNotifier::new(token, chat_id, endpoint));
                    } else {
                        eprintln!(
                            "Backend 'telegram' enabled, but no telegram configuration or environment variables found."
                        );
                    }
                }
                #[cfg(not(feature = "telegram"))]
                "telegram" => {
                    eprintln!(
                        "Backend 'telegram' requested, but rmd was compiled without the 'telegram' feature."
                    );
                }

                other => {
                    eprintln!("Unknown notification backend: '{}'", other);
                }
            }
        }

        dispatcher
    }

    /// Sends a notification to all configured and enabled backends.
    pub async fn send_all(&self, summary: &str, body: &str) {
        if self.notifiers.is_empty() {
            eprintln!(
                "Warning: No active notification backends configured. Reminder not displayed."
            );
            return;
        }

        for notifier in &self.notifiers {
            if let Err(e) = notifier.send(summary, body).await {
                eprintln!("Failed to send notification via {}: {}", notifier.name(), e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockNotifier {
        name: &'static str,
        sent: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl Notifier for MockNotifier {
        fn name(&self) -> &'static str {
            self.name
        }

        fn send<'a>(
            &'a self,
            summary: &'a str,
            body: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
            let sent = Arc::clone(&self.sent);
            let s = summary.to_string();
            let b = body.to_string();
            Box::pin(async move {
                sent.lock().unwrap().push((s, b));
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn test_dispatcher_dispatches_to_all_notifiers() {
        let mut dispatcher = NotificationDispatcher::new();
        let sent1 = Arc::new(Mutex::new(Vec::new()));
        let sent2 = Arc::new(Mutex::new(Vec::new()));

        dispatcher.add(MockNotifier {
            name: "Mock1",
            sent: Arc::clone(&sent1),
        });
        dispatcher.add(MockNotifier {
            name: "Mock2",
            sent: Arc::clone(&sent2),
        });

        dispatcher.send_all("Meeting", "Team sync").await;

        assert_eq!(sent1.lock().unwrap().len(), 1);
        assert_eq!(
            sent1.lock().unwrap()[0],
            ("Meeting".to_string(), "Team sync".to_string())
        );
        assert_eq!(sent2.lock().unwrap().len(), 1);
        assert_eq!(
            sent2.lock().unwrap()[0],
            ("Meeting".to_string(), "Team sync".to_string())
        );
    }
}
