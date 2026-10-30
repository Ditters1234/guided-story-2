use super::*;
#[derive(Debug, Clone)]
pub enum ComfySubmission {
    Queued(String),
    Rejected(String),
    Unknown(String),
}
#[derive(Debug, Clone)]
pub enum ComfyResult {
    Pending,
    Failed(String),
    Image(Vec<u8>),
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImagePreflight {
    pub ready: bool,
    pub issues: Vec<String>,
}
pub(crate) fn validate_remote_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 100
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid ComfyUI prompt ID.".into());
    }
    Ok(())
}
fn client(seconds: u32) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(std::time::Duration::from_secs(seconds as u64))
        .build()
        .map_err(|_| "Could not initialize the image API client.".into())
}
async fn bounded(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    if response.content_length().is_some_and(|v| v > limit as u64) {
        return Err("ComfyUI response exceeds its size limit.".into());
    }
    let mut bytes = vec![];
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "ComfyUI response was interrupted.")?
    {
        if bytes.len() + chunk.len() > limit {
            return Err("ComfyUI response exceeds its size limit.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
async fn get_json(
    endpoint: &str,
    route: &str,
    timeout: u32,
    limit: usize,
) -> Result<Value, String> {
    let response = client(timeout)?
        .get(image_endpoint(endpoint, route)?)
        .send()
        .await
        .map_err(|_| "Could not reach ComfyUI. Check its address and server status.")?;
    if !response.status().is_success() {
        return Err(format!(
            "ComfyUI returned HTTP {}.",
            response.status().as_u16()
        ));
    }
    serde_json::from_slice(&bounded(response, limit).await?)
        .map_err(|_| "ComfyUI returned invalid JSON.".into())
}
pub fn validate_comfy_objects(graph: &Value, output: &str, objects: &Value) -> ImagePreflight {
    let mut issues = BTreeSet::new();
    if let Some(nodes) = graph.as_object() {
        for (id, node) in nodes {
            let class = node["class_type"].as_str().unwrap_or("");
            let Some(info) = objects.get(class) else {
                issues.insert(format!("Missing node class: {class}"));
                continue;
            };
            if id == output && info["output_node"] != true {
                issues.insert(format!(
                    "Output class {class} does not declare an output node."
                ));
            }
            if let Some(inputs) = node["inputs"].as_object() {
                for (name, value) in inputs {
                    let spec = info["input"]["required"]
                        .get(name)
                        .or_else(|| info["input"]["optional"].get(name));
                    if let Some(choices) = spec.and_then(|s| s.get(0)).and_then(Value::as_array) {
                        if !value.is_array() && !choices.contains(value) {
                            issues.insert(format!(
                                "{class}.{name}: installed choices do not include {value}"
                            ));
                        }
                    }
                }
            }
        }
    } else {
        issues.insert("The workflow is not an API graph.".into());
    }
    ImagePreflight {
        ready: issues.is_empty(),
        issues: issues.into_iter().take(100).collect(),
    }
}
pub async fn preflight_image(
    settings: &ImageConnection,
    graph: Option<&Value>,
) -> Result<ImagePreflight, String> {
    settings.validate()?;
    let stats = get_json(
        &settings.base_url,
        "system_stats",
        settings.timeout_seconds,
        1_000_000,
    )
    .await?;
    if !stats.is_object() {
        return Err("ComfyUI returned invalid system statistics.".into());
    }
    let objects = get_json(
        &settings.base_url,
        "object_info",
        settings.timeout_seconds,
        12_000_000,
    )
    .await?;
    Ok(validate_comfy_objects(
        graph.unwrap_or(&settings.profile.workflow),
        &settings.profile.bindings.output_node,
        &objects,
    ))
}
pub async fn submit_comfy(submission: &MediaSubmission, client_id: &str) -> ComfySubmission {
    let result=async{let response=client(submission.timeout_seconds)?.post(image_endpoint(&submission.endpoint,"prompt")?).json(&json!({"prompt":submission.graph,"client_id":client_id})).send().await.map_err(|_|"Submission outcome is uncertain: no complete reply arrived. Recover the prompt ID from ComfyUI; do not resubmit this candidate.")?;let status=response.status();let bytes=bounded(response,1_000_000).await?;if status.as_u16()==400{let message=serde_json::from_slice::<Value>(&bytes).ok().and_then(|v|v.get("error").cloned()).map(|v|v.to_string()).unwrap_or_else(||"Workflow validation failed".into());return Ok(ComfySubmission::Rejected(format!("ComfyUI rejected the workflow: {}",message.chars().take(2000).collect::<String>())));}if !status.is_success(){return Err(format!("ComfyUI returned HTTP {}; submission outcome is uncertain. Recover its prompt ID before starting another candidate.",status.as_u16()));}let value:Value=serde_json::from_slice(&bytes).map_err(|_|"Submission outcome is uncertain: invalid ComfyUI reply.")?;let id=value["prompt_id"].as_str().ok_or("Submission outcome is uncertain: ComfyUI did not return a prompt ID.")?;validate_remote_id(id)?;Ok::<_,String>(ComfySubmission::Queued(id.into()))}.await;
    result.unwrap_or_else(ComfySubmission::Unknown)
}
fn image_query(entry: &Value) -> Result<(String, String, String), String> {
    let filename = entry["filename"]
        .as_str()
        .ok_or("ComfyUI output has no filename.")?;
    let subfolder = entry["subfolder"].as_str().unwrap_or("");
    let kind = entry["type"].as_str().unwrap_or("output");
    if filename.is_empty()
        || filename.len() > 500
        || filename.contains(['/', '\\', ':'])
        || filename == ".."
        || subfolder.len() > 1000
        || subfolder.contains(['\\', ':'])
        || subfolder.starts_with('/')
        || subfolder.split('/').any(|s| s == "..")
        || !["output", "temp"].contains(&kind)
    {
        return Err("ComfyUI returned an unsafe or unsupported output reference.".into());
    }
    Ok((filename.into(), subfolder.into(), kind.into()))
}
pub async fn check_comfy(submission: &MediaSubmission) -> Result<ComfyResult, String> {
    let id = submission
        .prompt_id
        .as_deref()
        .ok_or("Recover the ComfyUI prompt ID first.")?;
    validate_remote_id(id)?;
    let history = get_json(
        &submission.endpoint,
        &format!("history/{id}"),
        submission.timeout_seconds,
        4_000_000,
    )
    .await?;
    let Some(record) = history.get(id) else {
        return Ok(ComfyResult::Pending);
    };
    if record["prompt"].get(2) != Some(&submission.graph) {
        return Err(
            "The recovered ComfyUI job does not match this candidate's exact submitted workflow."
                .into(),
        );
    }
    let status = &record["status"];
    if status["status_str"] == "error"
        || status["messages"].as_array().is_some_and(|m| {
            m.iter()
                .any(|v| v[0] == "execution_error" || v[0] == "execution_interrupted")
        })
    {
        return Ok(ComfyResult::Failed("ComfyUI stopped or failed this job. Its workflow and prompt were retained; create a new candidate to reroll.".into()));
    }
    if status["completed"] != true {
        return Ok(ComfyResult::Pending);
    }
    let images=record["outputs"][&submission.profile.bindings.output_node]["images"].as_array().ok_or("The configured output node did not return images. Check its custom saver and output binding.")?;
    if images.len() != 1 {
        return Err("This image slice expects exactly one image from the bound output node. Use batch size 1 in the workflow.".into());
    }
    let (filename, folder, kind) = image_query(&images[0])?;
    let mut url = image_endpoint(&submission.endpoint, "view")?;
    url.query_pairs_mut()
        .append_pair("filename", &filename)
        .append_pair("subfolder", &folder)
        .append_pair("type", &kind);
    let response = client(submission.timeout_seconds)?
        .get(url)
        .send()
        .await
        .map_err(|_| "Could not download the finished image; check this job again.")?;
    if !response.status().is_success() {
        return Err(format!(
            "ComfyUI image download returned HTTP {}. Check this job again.",
            response.status().as_u16()
        ));
    }
    let bytes = bounded(response, MAX_IMAGE).await?;
    decode_image(&bytes)?;
    Ok(ComfyResult::Image(bytes))
}
