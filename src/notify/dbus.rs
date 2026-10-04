use super::Notifier;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

pub struct DbusNotifier {
    service: String,
}

impl DbusNotifier {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }
}

impl Notifier for DbusNotifier {
    fn name(&self) -> &'static str {
        "D-Bus"
    }

    fn send<'a>(
        &'a self,
        summary: &'a str,
        body: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        let service = self.service.clone();
        let summary = summary.to_string();
        let body = body.to_string();

        Box::pin(async move {
            let connection = zbus::Connection::session().await?;
            let mut hints = std::collections::HashMap::new();
            hints.insert("urgency", zbus::zvariant::Value::from(2u8));

            connection
                .call_method(
                    Some(service.as_str()),
                    "/org/freedesktop/Notifications",
                    Some("org.freedesktop.Notifications"),
                    "Notify",
                    &(
                        "rmd",
                        0u32,
                        "",
                        summary.as_str(),
                        body.as_str(),
                        Vec::<&str>::new(),
                        hints,
                        -1i32,
                    ),
                )
                .await?;
            Ok(())
        })
    }
}
