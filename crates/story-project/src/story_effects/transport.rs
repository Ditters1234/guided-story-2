use super::*;

/// Expand provider-only sparse node updates from the exact captured scene.
/// Review, manual JSON, receipts and acceptance continue to use complete nodes.
pub(crate) fn materialize_provider_effects(text: &str, captured: &World) -> Result<String, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let changes = value.get_mut("changes").ok_or("Missing changes.")?;
    let nodes = if changes.is_array() {
        changes.as_array_mut().unwrap()
    } else {
        changes
            .get_mut("putNodes")
            .and_then(Value::as_array_mut)
            .ok_or("changes.putNodes must be an array.")?
    };
    for change in nodes {
        if change["kind"] != "putNode" {
            continue;
        }
        let patch = change
            .get_mut("node")
            .and_then(Value::as_object_mut)
            .ok_or("putNode.node must be an object.")?;
        let id = patch
            .get("id")
            .and_then(Value::as_str)
            .ok_or("A node update needs its supplied ID.")?;
        if alias(id) {
            continue;
        }
        let saved = captured.nodes.iter().find(|n| n.id == id).ok_or("A node update is outside the captured scene. Add the intended record as a reference and check again.")?;
        let mut complete = serde_json::to_value(saved).map_err(|e| e.to_string())?;
        for (key, proposed) in patch.iter() {
            if key == "components" {
                let components = proposed.as_object().ok_or("An updated components value must be an object; capabilities cannot be removed by passage effects.")?;
                for (component, fields) in components {
                    let fields = fields.as_object().ok_or("Updated capability fields must be an object; use explicit null only for an optional field.")?;
                    if complete["components"].get(component).is_none() {
                        // Retain unsupported capability/field data for native diagnostics.
                        complete["components"][component] = json!({});
                    }
                    for (field, proposed) in fields {
                        complete["components"][component][field] = proposed.clone();
                    }
                }
            } else {
                // Unknown keys survive to strict parsing: never discard provider facts.
                complete[key] = proposed.clone();
            }
        }
        *patch = complete.as_object().unwrap().clone();
    }
    let expanded =
        materialize_effects_text(&serde_json::to_string(&value).map_err(|e| e.to_string())?)?;
    if expanded.len() > 500000 {
        return Err("Expanded proposals exceed the 500 KB draft limit. Narrow the selected records or proposals and check again; the complete original provider response is retained.".into());
    }
    Ok(expanded)
}
