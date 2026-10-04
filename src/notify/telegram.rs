use super::Notifier;
use anyhow::{Context, Result, bail};
use std::future::Future;
use std::pin::Pin;

pub struct TelegramNotifier {
    bot_token: String,
    chat_id: String,
    endpoint: String,
    client: reqwest::Client,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: String, endpoint: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            bot_token,
            chat_id,
            endpoint,
            client,
        }
    }
}

impl Notifier for TelegramNotifier {
    fn name(&self) -> &'static str {
        "Telegram"
    }

    fn send<'a>(
        &'a self,
        summary: &'a str,
        body: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        let endpoint = self.endpoint.trim_end_matches('/').to_string();
        let bot_token = self.bot_token.clone();
        let chat_id = self.chat_id.clone();
        let summary = summary.to_string();
        let body = body.to_string();
        let client = self.client.clone();

        Box::pin(async move {
            let url = format!("{}/bot{}/sendMessage", endpoint, bot_token);

            let text = if summary.is_empty() {
                escape_html(&body)
            } else if body.is_empty() {
                format!("<b>{}</b>", escape_html(&summary))
            } else {
                format!("<b>{}</b>\n{}", escape_html(&summary), escape_html(&body))
            };

            let payload = serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "HTML"
            });

            let res = client
                .post(&url)
                .json(&payload)
                .send()
                .await
                .context("Failed to connect to Telegram endpoint")?;

            if !res.status().is_success() {
                let status = res.status();
                let err_text = res.text().await.unwrap_or_default();
                bail!(
                    "Telegram API responded with error status {}: {}",
                    status,
                    err_text
                );
            }

            Ok(())
        })
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(
            escape_html("Fish & Chips <fast>"),
            "Fish &amp; Chips &lt;fast&gt;"
        );
    }
}
