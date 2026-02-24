// HTTP Request Handlers for AGI Lab Interface

use actix_web::{web, Responder, HttpResponse, http::StatusCode};
use serde::{Serialize, Deserialize};
use tracing::info;
use anyhow::Result;

// API Request/Response Models

#[derive(Serialize, Deserialize, Debug)]
pub struct AgiCommandRequest {
    pub command: String,
    pub args: Option<serde_json::Value>,
    pub session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AgiCommandResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub session_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AgiStatusResponse {
    pub connected: bool,
    pub session_id: Option<String>,
    pub providers: Vec<String>,
    pub models: Vec<String>,
    pub current_model: Option<String>},

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    pub id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// API Endpoints

pub async fn handle_agi_command(
    req: web::Json<AgiCommandRequest>,
) -> impl Responder {
    info!("Received AGI command: {:?}", req.command);

    let response = AgiCommandResponse {
        success: true,
        message: format!("Command '{}' received successfully", req.command),
        data: Some(serde_json::json!({ "status": "processing" })),
        session_id: req.session_id.clone(),
        error: None,
    };

    HttpResponse::Ok().json(response)
}

pub async fn handle_agi_status() -> impl Responder {
    let response = AgiStatusResponse {
        connected: true,
        session_id: Some("session-123".to_string()),
        providers: vec!["anthropic".to_string(), "openrouter".to_string()],
        models: vec!["claude-3-5-sonnet".to_string(), "gpt-4".to_string()],
        current_model: Some("claude-3-5-sonnet".to_string()),
    };

    HttpResponse::Ok().json(response)
}

pub async fn handle_json_rpc(
    req: web::Json<JsonRpcRequest>,
) -> impl Responder {
    info!("Received JSON-RPC request: {}", req.method);

    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({ "status": "success", "method": req.method })),
        error: None,
        id: req.id.clone(),
    };

    HttpResponse::Ok().json(response)
}

pub async fn handle_health_check() -> impl Responder {
    HttpResponse::build(StatusCode::OK)
        .body("AGI Lab Interface is running")
}

pub async fn handle_not_found() -> impl Responder {
    HttpResponse::build(StatusCode::NOT_FOUND)
        .body("Endpoint not found")
}

pub async fn handle_error(err: actix_web::error::Error) -> impl Responder {
    HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
        .body(format!("Server error: {}", err))
}