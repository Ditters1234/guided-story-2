//! Model-specific ComfyUI API graphs with explicit, non-destructive input bindings.
//! This prepares data only; submission, jobs and saved media belong to the media service.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;

/// JSON numbers cross the WebView bridge as JavaScript numbers. Workflows reject
/// integers beyond the safe range rather than silently rounding saved graphs.
#[derive(Type, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkflowJson {
    Null,
    Boolean(bool),
    Number(f64),
    Text(String),
    Array(Vec<WorkflowJson>),
    Object(std::collections::BTreeMap<String, WorkflowJson>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowInput {
    pub node_id: String,
    pub input: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowBindings {
    pub prompt: WorkflowInput,
    pub negative: Option<WorkflowInput>,
    pub seed: Option<WorkflowInput>,
    pub width: Option<WorkflowInput>,
    pub height: Option<WorkflowInput>,
    pub steps: Option<WorkflowInput>,
    pub cfg: Option<WorkflowInput>,
    pub output_node: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageWorkflowProfile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub family: String,
    #[specta(type = WorkflowJson)]
    pub workflow: Value,
    pub bindings: WorkflowBindings,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageWorkflowRequest {
    pub prompt: String,
    pub negative: Option<String>,
    pub seed: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub steps: Option<u32>,
    pub cfg: Option<f64>,
}

pub fn parse_comfy_workflow(text: &str) -> Result<Value, String> {
    if text.len() > 1_000_000 {
        return Err("Workflow JSON exceeds 1 MB.".into());
    }
    let graph: Value = serde_json::from_str(text).map_err(|_| "Invalid ComfyUI workflow JSON.")?;
    validate_graph(&graph)?;
    Ok(graph)
}
fn validate_graph(graph: &Value) -> Result<(), String> {
    let mut values = vec![graph];
    while let Some(value) = values.pop() {
        match value{Value::Number(n) if n.as_f64().is_some_and(|v|v.abs()>9_007_199_254_740_991.0)=>return Err("Workflow numbers must stay within JavaScript's safe numeric range; use a smaller seed.".into()),Value::Array(a)=>values.extend(a),Value::Object(o)=>values.extend(o.values()),_=>()}
    }
    if graph.to_string().len() > 1_000_000 {
        return Err("Workflow JSON exceeds 1 MB.".into());
    }
    let nodes = graph
        .as_object()
        .ok_or("Use ComfyUI API-format JSON, not a canvas export.")?;
    if nodes.is_empty() || nodes.len() > 500 {
        return Err("An API workflow must contain 1–500 nodes.".into());
    }
    for (id, node) in nodes {
        if id.is_empty()
            || id.len() > 100
            || node["class_type"]
                .as_str()
                .is_none_or(|v| v.is_empty() || v.len() > 200)
        {
            return Err("Each workflow node needs an ID and class_type.".into());
        }
        let inputs = node["inputs"]
            .as_object()
            .ok_or("Each API workflow node needs an inputs object.")?;
        for value in inputs.values() {
            if let Some(link) = value
                .as_array()
                .filter(|v| v.len() == 2 && v[0].is_string() && v[1].is_u64())
            {
                if !nodes.contains_key(link[0].as_str().unwrap()) {
                    return Err("A workflow link refers to a missing node.".into());
                }
            }
        }
    }
    Ok(())
}
fn bound_value<'a>(graph: &'a Value, binding: &WorkflowInput) -> Result<&'a Value, String> {
    graph
        .get(&binding.node_id)
        .and_then(|n| n.get("inputs"))
        .and_then(|n| n.get(&binding.input))
        .ok_or_else(|| {
            format!(
                "Workflow binding {}.{} does not exist.",
                binding.node_id, binding.input
            )
        })
}
fn set_bound(graph: &mut Value, binding: &WorkflowInput, value: Value) -> Result<(), String> {
    let previous = bound_value(graph, binding)?;
    if !(previous.is_string() && value.is_string() || previous.is_number() && value.is_number()) {
        return Err(format!(
            "Workflow binding {}.{} has the wrong input type.",
            binding.node_id, binding.input
        ));
    }
    graph[&binding.node_id]["inputs"][&binding.input] = value;
    Ok(())
}
fn set_optional(
    graph: &mut Value,
    binding: &Option<WorkflowInput>,
    value: Option<Value>,
    label: &str,
) -> Result<(), String> {
    if let Some(value) = value {
        let binding = binding
            .as_ref()
            .ok_or_else(|| format!("This workflow has no {label} binding."))?;
        set_bound(graph, binding, value)?;
    }
    Ok(())
}
impl ImageWorkflowProfile {
    pub fn prepare(&self, request: &ImageWorkflowRequest) -> Result<Value, String> {
        if self.schema_version != 1 {
            return Err("Unsupported image workflow profile version.".into());
        }
        validate_graph(&self.workflow)?;
        if self.workflow.get(&self.bindings.output_node).is_none() {
            return Err("The selected workflow output node is missing.".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for (binding, text) in std::iter::once((&self.bindings.prompt, true))
            .chain(self.bindings.negative.iter().map(|b| (b, true)))
            .chain(
                [
                    &self.bindings.seed,
                    &self.bindings.width,
                    &self.bindings.height,
                    &self.bindings.steps,
                    &self.bindings.cfg,
                ]
                .into_iter()
                .flatten()
                .map(|b| (b, false)),
            )
        {
            let value = bound_value(&self.workflow, binding)?;
            let valid_type = if text {
                value.is_string()
            } else {
                value.is_number()
            };
            if !seen.insert((&binding.node_id, &binding.input)) || !valid_type {
                return Err(
                    "Workflow bindings must name distinct inputs with matching text/number types."
                        .into(),
                );
            }
        }
        if request.prompt.trim().is_empty()
            || request.prompt.chars().count() > 16000
            || request
                .negative
                .as_ref()
                .is_some_and(|s| s.chars().count() > 16000)
        {
            return Err("Use a nonempty image prompt of at most 16,000 characters.".into());
        }
        if request
            .width
            .into_iter()
            .chain(request.height)
            .any(|v| !(64..=4096).contains(&v) || v % 8 != 0)
            || request.steps.is_some_and(|v| !(1..=150).contains(&v))
            || request
                .cfg
                .is_some_and(|v| !v.is_finite() || !(0.0..=30.0).contains(&v))
        {
            return Err("Check image dimensions, steps and CFG for the selected workflow.".into());
        }
        let mut graph = self.workflow.clone();
        set_bound(&mut graph, &self.bindings.prompt, json!(request.prompt))?;
        let negative = request
            .negative
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| json!(s));
        set_optional(
            &mut graph,
            &self.bindings.negative,
            negative,
            "negative prompt",
        )?;
        for (binding, value, label) in [
            (&self.bindings.seed, request.seed.map(|v| json!(v)), "seed"),
            (
                &self.bindings.width,
                request.width.map(|v| json!(v)),
                "width",
            ),
            (
                &self.bindings.height,
                request.height.map(|v| json!(v)),
                "height",
            ),
            (
                &self.bindings.steps,
                request.steps.map(|v| json!(v)),
                "steps",
            ),
            (&self.bindings.cfg, request.cfg.map(|v| json!(v)), "CFG"),
        ] {
            set_optional(&mut graph, binding, value, label)?;
        }
        for (binding, label) in [
            (&self.bindings.width, "width"),
            (&self.bindings.height, "height"),
            (&self.bindings.steps, "steps"),
        ] {
            if let Some(binding) = binding {
                let value = bound_value(&graph, binding)?
                    .as_u64()
                    .ok_or("Workflow dimensions and steps must be whole numbers.")?;
                if if label == "steps" {
                    !(1..=150).contains(&value)
                } else {
                    !(64..=4096).contains(&value) || value % 8 != 0
                } {
                    return Err(format!(
                        "Check the workflow's default or overridden {label}."
                    ));
                }
            }
        }
        if let Some(binding) = &self.bindings.cfg {
            let value = bound_value(&graph, binding)?
                .as_f64()
                .ok_or("Workflow CFG must be a number.")?;
            if !(0.0..=30.0).contains(&value) {
                return Err("Check the workflow's default or overridden CFG.".into());
            }
        }
        Ok(graph)
    }
}
pub fn guided_story_image_workflow(desktop_layout: bool) -> Result<ImageWorkflowProfile, String> {
    let graph = if desktop_layout {
        include_str!("../../../workflows/guided-story/portrait.comfy-desktop.api.json")
    } else {
        include_str!("../../../workflows/guided-story/portrait.api.json")
    };
    let binding = |node: &str, input: &str| WorkflowInput {
        node_id: node.into(),
        input: input.into(),
    };
    Ok(ImageWorkflowProfile {
        schema_version: 1,
        id: if desktop_layout {
            "guided-story.krea2.desktop"
        } else {
            "guided-story.krea2.original"
        }
        .into(),
        name: "Guided Story Krea 2".into(),
        family: "krea2".into(),
        workflow: parse_comfy_workflow(graph)?,
        bindings: WorkflowBindings {
            prompt: binding("4", "text"),
            negative: None,
            seed: Some(binding("7", "seed")),
            width: Some(binding("6", "width")),
            height: Some(binding("6", "height")),
            steps: Some(binding("7", "steps")),
            cfg: Some(binding("7", "cfg")),
            output_node: "9".into(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guided_story_graphs_keep_model_lora_encoder_and_sampler_defaults() {
        for desktop in [false, true] {
            let profile = guided_story_image_workflow(desktop).unwrap();
            let graph = profile
                .prepare(&ImageWorkflowRequest {
                    prompt: "An adult botanist beside a window.".into(),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(graph["7"]["inputs"]["steps"], 9);
            assert_eq!(graph["7"]["inputs"]["cfg"], 1);
            assert_eq!(graph["6"]["inputs"]["width"], 896);
            assert_eq!(graph["6"]["inputs"]["height"], 1152);
            assert_eq!(graph["9"]["class_type"], "SeeSeeCivitaiSaveImage");
            for id in ["1", "2", "3", "5", "15"] {
                assert_eq!(graph[id], profile.workflow[id]);
            }
        }
    }
    #[test]
    fn overrides_change_only_bound_values_without_mutating_the_template() {
        let profile = guided_story_image_workflow(true).unwrap();
        let request = ImageWorkflowRequest {
            prompt: "A conservatory at dusk.".into(),
            seed: Some(123),
            width: Some(1152),
            height: Some(896),
            steps: Some(12),
            cfg: Some(1.2),
            negative: None,
        };
        let graph = profile.prepare(&request).unwrap();
        assert_eq!(graph["4"]["inputs"]["text"], request.prompt);
        assert_eq!(graph["7"]["inputs"]["seed"], 123);
        assert_eq!(
            graph["7"]["inputs"]["scheduler"],
            profile.workflow["7"]["inputs"]["scheduler"]
        );
        assert_eq!(profile.workflow["7"]["inputs"]["seed"], 42);
        assert_eq!(graph["15"], profile.workflow["15"]);
    }
    #[test]
    fn unsupported_negative_prompts_and_broken_graphs_fail_instead_of_silently_rewriting() {
        let profile = guided_story_image_workflow(true).unwrap();
        let request = ImageWorkflowRequest {
            prompt: "A room.".into(),
            negative: Some("text".into()),
            ..Default::default()
        };
        assert!(profile
            .prepare(&request)
            .unwrap_err()
            .contains("negative prompt"));
        assert!(parse_comfy_workflow(r#"{"nodes":[]}"#).is_err());
        assert!(parse_comfy_workflow(
            r#"{"1":{"class_type":"SaveImage","inputs":{"images":["missing",0]}}}"#
        )
        .is_err());
    }
    #[test]
    fn other_model_families_use_their_own_graph_inputs_without_hardcoded_loader_names() {
        let mut profile = guided_story_image_workflow(true).unwrap();
        profile.family = "custom".into();
        profile.workflow = json!({"paint":{"class_type":"CustomPainter","inputs":{"caption":"","noise":1}},"save":{"class_type":"SaveImage","inputs":{"images":["paint",0]}}});
        profile.bindings = WorkflowBindings {
            prompt: WorkflowInput {
                node_id: "paint".into(),
                input: "caption".into(),
            },
            negative: None,
            seed: Some(WorkflowInput {
                node_id: "paint".into(),
                input: "noise".into(),
            }),
            width: None,
            height: None,
            steps: None,
            cfg: None,
            output_node: "save".into(),
        };
        let graph = profile
            .prepare(&ImageWorkflowRequest {
                prompt: "A riverbank.".into(),
                seed: Some(7),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(graph["paint"]["inputs"]["caption"], "A riverbank.");
        assert_eq!(graph["paint"]["inputs"]["noise"], 7);
        assert!(profile
            .prepare(&ImageWorkflowRequest {
                prompt: "A riverbank.".into(),
                width: Some(1024),
                ..Default::default()
            })
            .is_err());
    }
}
