//! Opt-in diagnostics for a fictional captured request. Never runs in ordinary checks.
use super::*;
use crate::{connection_api_key, load_connection};
use std::{fs, path::Path, time::Instant};

#[tokio::test]
#[ignore = "explicit bounded streaming profile of a fictional local request"]
async fn local_creation_stream_profile() {
    let Ok(config) = std::env::var("STORY_CREATION_QUALITY_CONFIG") else {
        return;
    };
    let input =
        std::env::var("STORY_LIVE_PROFILE_INPUT").expect("Select the fictional captured request");
    let output =
        std::env::var("STORY_CREATION_QUALITY_ROOT").expect("Select a new diagnostic folder");
    let root = Path::new(&output);
    fs::create_dir(root).unwrap();
    let captured: Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    let mut settings = load_connection(Path::new(&config)).unwrap().settings;
    settings.timeout_seconds = 180;
    if std::env::var_os("STORY_CREATION_QUALITY_DIRECT_OFF").is_some() {
        settings.thinking = ThinkingControl::DirectOff;
    }
    let endpoint = url::Url::parse(&settings.base_url).unwrap();
    assert!(match endpoint.host() {
        Some(url::Host::Domain(s)) => s.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    });
    let key = connection_api_key(Path::new(&config), &settings).unwrap();
    let api = client(&settings).unwrap();
    let mut status_request = api.get(format!(
        "{}/status",
        settings.base_url.trim_end_matches('/')
    ));
    if let Some(key) = &key {
        status_request = status_request.bearer_auth(key);
    }
    if let Ok(response) = status_request.send().await {
        if let Ok(value) = response.json::<Value>().await {
            let status: serde_json::Map<_, _> = [
                "loaded",
                "loading",
                "active_model",
                "context_length",
                "supports_reasoning",
                "reasoning_always_on",
                "reasoning_style",
                "reasoning_budget",
                "gpu_layers",
                "offloaded_layers",
            ]
            .iter()
            .filter_map(|k| value.get(*k).map(|v| ((*k).into(), v.clone())))
            .collect();
            fs::write(
                root.join("server-status.json"),
                serde_json::to_vec_pretty(&status).unwrap(),
            )
            .unwrap();
        }
    }
    let user = serde_json::to_string(&captured["user"]).unwrap();
    let system = captured["system"].as_str().unwrap();
    let schema = &captured["user"]["responseSchema"];
    let mut payload = completion_payload(
        &settings,
        system,
        &user,
        Some(schema),
        settings.max_tokens.min(4096),
    );
    payload["stream"] = json!(true);
    payload["stream_options"] = json!({"include_usage":true});
    let mut request = api.post(settings.endpoint()).json(&payload);
    if let Some(key) = &key {
        request = request.bearer_auth(key);
    }
    let started = Instant::now();
    let mut first = None;
    let mut content = String::new();
    let mut reasoning_chars = 0usize;
    let mut usage = Value::Null;
    let mut finish = Value::Null;
    let mut error = String::new();
    let mut buffer = Vec::new();
    let mut last_report = 0u64;
    let mut done = false;
    let mut total = 0usize;
    match request.send().await {
        Err(e) => error = format!("Request failed: {e}"),
        Ok(mut response) => {
            if !response.status().is_success() {
                error = format!("HTTP {}", response.status());
            } else {
                loop {
                    let chunk = match response.chunk().await {
                        Ok(Some(c)) => c,
                        Ok(None) => break,
                        Err(e) => {
                            error = format!("Stream interrupted: {e}");
                            break;
                        }
                    };
                    total += chunk.len();
                    if total > 4000000 {
                        error = "Stream exceeds bounded diagnostic size".into();
                        break;
                    }
                    buffer.extend_from_slice(&chunk);
                    while let Some(end) = buffer.iter().position(|b| *b == b'\n') {
                        let line = String::from_utf8_lossy(&buffer[..end])
                            .trim_end_matches('\r')
                            .to_string();
                        buffer.drain(..=end);
                        let Some(data) = line.strip_prefix("data:") else {
                            continue;
                        };
                        let data = data.trim();
                        if data == "[DONE]" {
                            done = true;
                            break;
                        }
                        let Ok(value) = serde_json::from_str::<Value>(data) else {
                            continue;
                        };
                        let delta = &value["choices"][0]["delta"];
                        if let Some(text) = delta["content"].as_str() {
                            if !text.is_empty() && first.is_none() {
                                first = Some(started.elapsed().as_secs_f64());
                            }
                            content.push_str(text);
                        }
                        if let Some(text) = delta["reasoning_content"]
                            .as_str()
                            .or_else(|| delta["reasoning"].as_str())
                        {
                            reasoning_chars += text.len();
                        }
                        if !value["usage"].is_null() {
                            usage = value["usage"].clone();
                        }
                        if !value["choices"][0]["finish_reason"].is_null() {
                            finish = value["choices"][0]["finish_reason"].clone();
                        }
                    }
                    if started.elapsed().as_secs() >= last_report + 10 {
                        last_report = started.elapsed().as_secs();
                        println!(
                            "{}",
                            json!({"seconds":last_report,"contentChars":content.len(),"reasoningChars":reasoning_chars})
                        );
                    }
                    if done {
                        break;
                    }
                }
            }
        }
    }
    fs::write(root.join("response.txt"), &content).unwrap();
    let report = json!({"seconds":started.elapsed().as_secs_f64(),"firstContentSeconds":first,"contentChars":content.len(),"reasoningChars":reasoning_chars,"usage":usage,"finishReason":finish,"done":done,"error":error,"thinking":settings.thinking});
    fs::write(
        root.join("profile.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
    assert!(error.is_empty(), "{error}");
    assert!(done);
    assert!(!content.is_empty());
}
