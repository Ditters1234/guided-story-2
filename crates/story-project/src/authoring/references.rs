use super::*;
use crate::prompt_context::ReferenceScope;

pub(super) fn scope<'a>(
    source: &'a World,
    model: &World,
    context: &World,
    request: &AuthoringRequest,
    existing: Option<&ExistingAuthoringContext>,
) -> ReferenceScope<'a> {
    let ids = context
        .nodes
        .iter()
        .map(|n| n.id.clone())
        .chain(
            existing
                .into_iter()
                .flat_map(|c| c.catalog.iter().map(|n| n.id.clone())),
        )
        .collect();
    ReferenceScope::new(
        source,
        ids,
        (request.mode == AuthoringMode::Create).then_some(model),
        if request.bundle {
            creation::new_alias_schema()
        } else {
            json!({"const":"new:root"})
        },
    )
}

pub(super) fn bind(
    scope: &ReferenceScope,
    model: &World,
    request: &AuthoringRequest,
    settings: &LlmConnection,
    input: &mut Value,
    schema: &mut Value,
) {
    let used = scope.authoring(schema, model);
    creation::strip_schema_help(schema);
    if !used.is_empty() {
        input["referenceChoices"] = scope.guidance(
            &used,
            request.mode == AuthoringMode::Create
                && settings.structured_output != StructuredOutput::Schema,
        );
    }
    if settings.structured_output != StructuredOutput::Schema {
        input["responseSchema"] = if request.mode == AuthoringMode::Create {
            creation::prompt_schema(model, request)
        } else {
            schema.clone()
        };
    }
    if request.mode == AuthoringMode::Create
        && settings.structured_output == StructuredOutput::Schema
    {
        crate::prompt_context::share_component_schemas(schema);
    }
}

pub(super) fn schema_bytes(schema: &Value, settings: &LlmConnection) -> usize {
    if settings.structured_output == StructuredOutput::Schema {
        schema.to_string().len()
    } else {
        0
    }
}
