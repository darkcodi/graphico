//! MCP server for Graphico over stdio.
//! Set `GRAPHICO_API_URL` to override the default `http://127.0.0.1:3000`.

use anyhow::{Context, Result};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_API_URL: &str = "http://127.0.0.1:3000";

#[derive(Clone)]
struct GraphicoMcp {
    client: reqwest::Client,
    base: String,
    tool_router: ToolRouter<GraphicoMcp>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct Coordinates {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct GraphicoCreateNodeArgs {
    pub name: String,
    #[serde(default)]
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Neighbor node UUIDs (strings accepted by the HTTP API).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<String>>,
    pub position: Coordinates,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct GraphicoCreateNodesBulkArgs {
    pub nodes: Vec<GraphicoCreateNodeArgs>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct GraphicoUpdateNodeArgs {
    /// Node UUID to update (path parameter; required).
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Neighbor UUIDs; omit to keep current edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Coordinates>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct GraphicoNodeIdArgs {
    /// Node UUID returned by create or listed in the graph.
    pub id: String,
}

#[tool_router]
impl GraphicoMcp {
    fn new(client: reqwest::Client, base: String) -> Self {
        Self {
            client,
            base,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Create a new graph node. Returns JSON with the new node's id."
    )]
    async fn create_node(
        &self,
        Parameters(args): Parameters<GraphicoCreateNodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let edges = parse_uuid_list_opt(args.edges.as_ref())?;
        let url = format!("{}/nodes", self.base.trim_end_matches('/'));
        let body = serde_json::json!({
            "name": args.name,
            "data": args.data,
            "color": args.color,
            "edges": edges,
            "position": { "x": args.position.x, "y": args.position.y },
        });
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| http_client_error(e))?;
        tool_result_from_response(resp).await
    }

    #[tool(
        description = "Create multiple graph nodes in one request. Returns JSON with `ids` in the same order as the input."
    )]
    async fn create_nodes_bulk(
        &self,
        Parameters(args): Parameters<GraphicoCreateNodesBulkArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut nodes = Vec::with_capacity(args.nodes.len());
        for node in &args.nodes {
            let edges = parse_uuid_list_opt(node.edges.as_ref())?;
            nodes.push(serde_json::json!({
                "name": node.name,
                "data": node.data,
                "color": node.color,
                "edges": edges,
                "position": { "x": node.position.x, "y": node.position.y },
            }));
        }
        let body = serde_json::json!({ "nodes": nodes });
        let url = format!("{}/nodes/bulk", self.base.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| http_client_error(e))?;
        tool_result_from_response(resp).await
    }

    #[tool(description = "Fetch a single graph node by UUID.")]
    async fn get_node(
        &self,
        Parameters(args): Parameters<GraphicoNodeIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&args.id)?;
        let url = format!("{}/nodes/{}", self.base.trim_end_matches('/'), id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| http_client_error(e))?;
        tool_result_from_response(resp).await
    }

    #[tool(description = "List all graph nodes.")]
    async fn list_nodes(&self) -> Result<CallToolResult, McpError> {
        let url = format!("{}/nodes", self.base.trim_end_matches('/'));
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| http_client_error(e))?;
        tool_result_from_response(resp).await
    }

    #[tool(
        description = "Partially update a graph node. Only `id` is required; omit other fields to leave them unchanged."
    )]
    async fn update_node(
        &self,
        Parameters(args): Parameters<GraphicoUpdateNodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&args.id)?;
        let url = format!("{}/nodes/{}", self.base.trim_end_matches('/'), id);
        let body = build_partial_update_body(&args)?;
        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| http_client_error(e))?;
        tool_result_from_response(resp).await
    }

    #[tool(description = "Delete a graph node by UUID.")]
    async fn delete_node(
        &self,
        Parameters(args): Parameters<GraphicoNodeIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let id = parse_uuid(&args.id)?;
        let url = format!("{}/nodes/{}", self.base.trim_end_matches('/'), id);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| http_client_error(e))?;
        tool_result_from_response(resp).await
    }

    #[tool(
        description = "Delete every node in the graph (no-op if the graph is already empty)."
    )]
    async fn delete_all_nodes(&self) -> Result<CallToolResult, McpError> {
        let url = format!("{}/nodes", self.base.trim_end_matches('/'));
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| http_client_error(e))?;
        tool_result_from_response(resp).await
    }
}

#[tool_handler]
impl ServerHandler for GraphicoMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(
            "Run the Graphico app so the server is reachable (default http://127.0.0.1:3000). Override with GRAPHICO_API_URL."
                .to_string(),
        )
    }
}

fn http_client_error(err: reqwest::Error) -> McpError {
    McpError::internal_error(err.to_string(), None)
}

fn parse_uuid(s: &str) -> Result<Uuid, McpError> {
    Uuid::parse_str(s.trim()).map_err(|e| {
        McpError::invalid_params(format!("invalid UUID `{s}`: {e}"), None)
    })
}

fn parse_uuid_list_opt(edges: Option<&Vec<String>>) -> Result<Option<Vec<Uuid>>, McpError> {
    let Some(list) = edges else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(list.len());
    for s in list {
        out.push(parse_uuid(s)?);
    }
    Ok(Some(out))
}

/// JSON body for PUT: only keys that should change (partial update).
fn build_partial_update_body(args: &GraphicoUpdateNodeArgs) -> Result<serde_json::Value, McpError> {
    let mut map = serde_json::Map::new();
    if let Some(ref n) = args.name {
        map.insert("name".into(), serde_json::json!(n));
    }
    if let Some(ref d) = args.data {
        map.insert("data".into(), serde_json::json!(d));
    }
    if let Some(ref c) = args.color {
        map.insert("color".into(), serde_json::json!(c));
    }
    if let Some(ref e) = args.edges {
        let uuids = parse_uuid_list_opt(Some(e))?.unwrap_or_default();
        map.insert(
            "edges".into(),
            serde_json::to_value(uuids).map_err(|e| McpError::internal_error(e.to_string(), None))?,
        );
    }
    if let Some(ref p) = args.position {
        map.insert(
            "position".into(),
            serde_json::json!({ "x": p.x, "y": p.y }),
        );
    }
    Ok(serde_json::Value::Object(map))
}

async fn tool_result_from_response(resp: reqwest::Response) -> Result<CallToolResult, McpError> {
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let text = if body.is_empty() {
        format!("HTTP {}", status)
    } else {
        format!("HTTP {}\n{}", status, body)
    };

    if status.is_success() {
        Ok(CallToolResult::success(vec![Content::text(text)]))
    } else {
        Ok(CallToolResult::error(vec![Content::text(text)]))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let base = std::env::var("GRAPHICO_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    let base = base
        .parse::<reqwest::Url>()
        .map(|u| u.to_string())
        .unwrap_or(base);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("build HTTP client")?;

    tracing::info!(%base, "graphico-mcp listening on stdio");

    let service = GraphicoMcp::new(client, base)
        .serve(rmcp::transport::stdio())
        .await
        .context("start MCP server")?;

    service.waiting().await.context("MCP server stopped")?;
    Ok(())
}
