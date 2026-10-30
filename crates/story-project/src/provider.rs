use crate::{LlmConnection, StructuredOutput, ThinkingControl, TokenParameter};
use serde_json::{json, Value};

pub struct CompletionText {
    pub text: String,
    pub incomplete: bool,
}
fn client(settings: &LlmConnection) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(std::time::Duration::from_secs(
            settings.timeout_seconds as u64,
        ))
        .build()
        .map_err(|_| "Could not initialize the text API client.".into())
}

/// Test credentials/connectivity with the model catalogue, without generating text.
pub async fn discover_models(
    settings: &LlmConnection,
    key: Option<&str>,
) -> Result<Vec<String>, String> {
    settings.validate()?;
    let mut request = client(settings)?.get(settings.models_endpoint());
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    parse_models(&send_bounded(request).await?)
}

fn parse_models(bytes: &[u8]) -> Result<Vec<String>, String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| "The provider did not return a valid model list.")?;
    let entries = value["data"]
        .as_array()
        .ok_or("The provider did not return a model list with data[].id entries.")?;
    if entries.len() > 1000 {
        return Err("The provider returned too many models (limit 1,000).".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut models = vec![];
    for entry in entries {
        let id = entry["id"]
            .as_str()
            .ok_or("A returned model is missing its ID.")?;
        if id.trim().is_empty() || id.len() > 200 || id.chars().any(char::is_control) {
            return Err("The provider returned an invalid model ID.".into());
        }
        if seen.insert(id) {
            models.push(id.to_string());
        }
    }
    if models.is_empty() {
        return Err("Connected, but the server returned no models. Load a model on the server and test again.".into());
    }
    Ok(models)
}
pub async fn complete_text(
    settings: &LlmConnection,
    key: Option<&str>,
    system: &str,
    user: &str,
    schema: Option<&Value>,
    output_tokens: u32,
) -> Result<CompletionText, String> {
    settings.ready()?;
    let client = client(settings)?;
    let payload = completion_payload(settings, system, user, schema, output_tokens);
    let mut request = client.post(settings.endpoint()).json(&payload);
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    parse_completion(&send_bounded(request).await?)
}

fn completion_payload(
    settings: &LlmConnection,
    system: &str,
    user: &str,
    schema: Option<&Value>,
    output_tokens: u32,
) -> Value {
    let mut payload = json!({"model":settings.model,"temperature":settings.temperature,"top_p":settings.top_p,"stream":false,"messages":[{"role":"system","content":system},{"role":"user","content":user}]});
    if let Some(value) = settings.top_k {
        payload["top_k"] = json!(value);
    }
    if let Some(value) = settings.min_p {
        payload["min_p"] = json!(value);
    }
    match settings.thinking {
        ThinkingControl::Provider => (),
        ThinkingControl::ChatTemplateOff => {
            payload["chat_template_kwargs"] = json!({"enable_thinking":false})
        }
        ThinkingControl::DirectOff => payload["enable_thinking"] = json!(false),
    }
    payload[match settings.token_parameter {
        TokenParameter::MaxTokens => "max_tokens",
        TokenParameter::MaxCompletionTokens => "max_completion_tokens",
    }] = json!(output_tokens);
    if let Some(schema) = schema {
        match settings.structured_output {
            StructuredOutput::Json => payload["response_format"] = json!({"type":"json_object"}),
            StructuredOutput::Schema => {
                payload["response_format"] = json!({"type":"json_schema","json_schema":{"name":"world_nodes","strict":false,"schema":schema}})
            }
            StructuredOutput::Prompt => (),
        }
    }
    payload
}

async fn send_bounded(request: reqwest::RequestBuilder) -> Result<Vec<u8>, String> {
    let mut response = request.send().await.map_err(|e| {
        if e.is_timeout() {
            "The text request timed out. No automatic retry was made."
        } else {
            "Could not reach the text API. Check the URL and server."
        }
    })?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err("The provider rejected the API credentials. Check Settings → LLM.".into());
    }
    if !response.status().is_success() {
        return Err(format!(
            "The text provider returned HTTP {}. No automatic retry or provider switch was made.",
            response.status().as_u16()
        ));
    }
    let mut bytes = vec![];
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "The text response was interrupted.")?
    {
        if bytes.len() + chunk.len() > 600000 {
            return Err("The text response exceeds its size limit.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
pub fn parse_completion(bytes: &[u8]) -> Result<CompletionText, String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| "The provider did not return a valid Chat Completions response.")?;
    let choice = &value["choices"][0];
    if !choice["message"]["refusal"].is_null() {
        return Err("The provider declined this request. Existing data was kept.".into());
    }
    let content = &choice["message"]["content"];
    let text = if let Some(text) = content.as_str() {
        text.to_string()
    } else {
        content
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter(|p| p["type"] == "text")
                    .filter_map(|p| p["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default()
    };
    let mut text = text.trim();
    if text.starts_with("<think>") {
        let end = text
            .find("</think>")
            .ok_or("The provider returned unfinished reasoning without usable output.")?;
        text = text[end + 8..].trim();
    }
    if text.starts_with("```") {
        let first = text
            .find('\n')
            .ok_or("The provider returned an empty code block.")?;
        text = text[first + 1..].trim();
        if let Some(stripped) = text.strip_suffix("```") {
            text = stripped.trim();
        }
    }
    if text.is_empty() || text.len() > 500000 {
        return Err("The provider returned no usable bounded text.".into());
    }
    Ok(CompletionText {
        text: text.into(),
        incomplete: choice["finish_reason"] == "length",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[test]
    fn qwen_sampling_and_thinking_transports_are_optional_without_prompt_hacks() {
        let mut settings = LlmConnection::default();
        let body = completion_payload(&settings, "System", "Author direction", None, 256);
        assert_eq!(body["temperature"], 0.7);
        assert_eq!(body["top_p"], 0.8);
        assert_eq!(body["top_k"], 20);
        assert_eq!(body["min_p"], 0.0);
        assert_eq!(body["chat_template_kwargs"]["enable_thinking"], false);
        assert_eq!(body["messages"][1]["content"], "Author direction");
        settings.thinking = ThinkingControl::DirectOff;
        let body = completion_payload(&settings, "System", "Author direction", None, 256);
        assert_eq!(body["enable_thinking"], false);
        assert!(body.get("chat_template_kwargs").is_none());
        settings.thinking = ThinkingControl::Provider;
        settings.top_k = None;
        settings.min_p = None;
        let body = completion_payload(&settings, "System", "Author direction", None, 256);
        for key in ["enable_thinking", "chat_template_kwargs", "top_k", "min_p"] {
            assert!(body.get(key).is_none());
        }
    }
    #[test]
    fn model_catalogues_are_bounded_and_preserve_server_order_without_duplicates() {
        assert_eq!(
            parse_models(br#"{"data":[{"id":"local-b"},{"id":"local-a"},{"id":"local-b"}]}"#)
                .unwrap(),
            vec!["local-b", "local-a"]
        );
        for invalid in [
            br#"{"data":[]}"#.as_slice(),
            br#"{"models":[]}"#,
            br#"{"data":[{}]}"#,
            br#"{"data":[{"id":""}]}"#,
        ] {
            assert!(parse_models(invalid).is_err());
        }
        assert!(parse_models(
            &serde_json::to_vec(&json!({"data":vec![json!({"id":"fixture"});1001]})).unwrap()
        )
        .is_err());
    }
    #[tokio::test]
    async fn connection_test_discovers_models_with_saved_key_without_requiring_a_model_or_generating(
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![];
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).await.unwrap();
                assert_ne!(n, 0);
                request.extend_from_slice(&chunk[..n]);
                if request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = r#"{"data":[{"id":"fictional-server-model"}]}"#;
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        });
        let mut settings = LlmConnection::default();
        settings.base_url = format!("http://127.0.0.1:{port}/v1/chat/completions/");
        assert!(settings.model.is_empty());
        let models = discover_models(&settings, Some("fictional-fixture-key"))
            .await
            .unwrap();
        assert_eq!(models, vec!["fictional-server-model"]);
        let request = server.await.unwrap();
        assert!(request.starts_with("GET /v1/models HTTP/1.1\r\n"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer fictional-fixture-key"));
        assert!(!request.contains("messages"));
        assert!(!request.contains("chat/completions"));
    }
    #[test]
    fn completion_envelopes_preserve_truncation_and_strip_only_complete_wrappers() {
        let parsed=parse_completion(br#"{"choices":[{"message":{"content":"<think>private provider analysis</think>```json\n{\"nodes\":[],\"edges\":[]}\n```"},"finish_reason":"length"}]}"#).unwrap();
        assert!(parsed.incomplete);
        assert_eq!(parsed.text, "{\"nodes\":[],\"edges\":[]}");
        assert!(parse_completion(
            br#"{"choices":[{"message":{"refusal":"declined","content":""}}]}"#
        )
        .is_err());
    }
    #[tokio::test]
    async fn compatible_http_has_bounded_one_call_payload_and_does_not_follow_redirects() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![];
            let mut chunk = [0; 8192];
            loop {
                let n = stream.read(&mut chunk).await.unwrap();
                request.extend_from_slice(&chunk[..n]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|v| v.parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/should-not-be-called\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
            String::from_utf8(request).unwrap()
        });
        let mut settings = LlmConnection::default();
        settings.model = "fixture-model".into();
        settings.base_url = format!("http://127.0.0.1:{port}/v1");
        let error = complete_text(
            &settings,
            None,
            "Return JSON",
            "Fixture prompt",
            Some(&json!({"type":"object"})),
            128,
        )
        .await
        .err()
        .unwrap();
        assert!(error.contains("302"));
        let request = server.await.unwrap();
        assert!(request.starts_with("POST /v1/chat/completions"));
        assert!(request.contains("\"response_format\":{\"type\":\"json_object\"}"));
        assert!(request.contains("\"max_tokens\":128"));
        assert!(!request.contains("Authorization"));
    }
}
