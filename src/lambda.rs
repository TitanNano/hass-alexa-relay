use anyhow::{anyhow, Context, Result};
use lambda_runtime::LambdaEvent;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    StatusCode,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct LambdaPayload {
    directive: LambdaDirective,
}

#[derive(Deserialize)]
struct LambdaDirective {
    header: LambdaDirectiveHeaders,
    endpoint: Option<LambdaDirectiveEndpoint>,
    payload: Option<LambdaDirectivePayload>,
}

#[derive(Deserialize)]
struct LambdaDirectiveHeaders {
    #[serde(rename = "payloadVersion")]
    payload_version: String,
}

#[derive(Deserialize)]
struct LambdaDirectiveEndpoint {
    scope: LambdaDirectiveScope,
}

#[derive(Deserialize)]
struct LambdaDirectivePayload {
    grantee: Option<LambdaDirectiveScope>,
    scope: Option<LambdaDirectiveScope>,
}

#[derive(Deserialize)]
struct LambdaDirectiveScope {
    #[serde(rename = "type")]
    ty: String,
    token: Option<String>,
}

/// Handle incoming Alexa directive.
pub async fn lambda_handler(
    event: LambdaEvent<serde_json::Value>,
    access_token: Option<String>,
) -> Result<serde_json::Value> {
    let (event, _context) = event.into_parts();
    let payload: LambdaPayload =
        serde_json::from_value(event.clone()).context("Failed to parse event payload")?;

    let base_url = "http://127.0.0.1:8080"; // os.environ.get('BASE_URL')

    if payload.directive.header.payload_version != "3" {
        return Err(anyhow!("only payload version 3 is supported!"));
    }

    let scope = payload
        .directive
        .endpoint
        .as_ref()
        .map(|e| &e.scope)
        .or_else(|| {
            payload
                .directive
                .payload
                .as_ref()
                .and_then(|p| p.grantee.as_ref())
        })
        .or_else(|| {
            payload
                .directive
                .payload
                .as_ref()
                .and_then(|p| p.scope.as_ref())
        })
        .ok_or_else(|| anyhow!("missing endpoint.scope"))?;

    if scope.ty != "BearerToken" {
        return Err(anyhow!("only BearerToken is supported"));
    }

    let token = scope
        .token
        .clone()
        .or_else(|| access_token.map(|t| t.to_owned()))
        .ok_or_else(|| anyhow!("access token missing!"))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();

    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {token}"))
            .context("Failed to create auth header")?,
    );
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let has_event_payload =
        serde_json::to_string(&event).context("Failed to serialize event payload!")?;

    let response = client
        .post(format!("{base_url}/api/alexa/smart_home"))
        .headers(headers)
        .body(has_event_payload)
        .send()
        .await
        .context("Failed to send upstream hass request!")?;

    let error_type = match response.status() {
        StatusCode::OK
        | StatusCode::CREATED
        | StatusCode::ACCEPTED
        | StatusCode::NON_AUTHORITATIVE_INFORMATION
        | StatusCode::NO_CONTENT
        | StatusCode::RESET_CONTENT
        | StatusCode::PARTIAL_CONTENT
        | StatusCode::MULTI_STATUS
        | StatusCode::ALREADY_REPORTED => None,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            Some("INVALID_AUTHORIZATION_CREDENTIAL")
        }
        _ => Some("INTERNAL_ERROR"),
    };

    let message = response
        .text_with_charset("utf-8")
        .await
        .context("Failed to decode hass response")?;
    let parsed_message: serde_json::Value =
        serde_json::from_str(&message).context("Failed to parse response message")?;

    if let Some(error_type) = error_type {
        let err = json!({
            "event": {
                "payload": {
                    "type": error_type,
                    "message": message
                }
            }
        });

        return Ok(err);
    }

    log::info!("response from hass: {:?}", parsed_message);
    Ok(parsed_message)
}
