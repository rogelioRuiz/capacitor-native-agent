//! Types exposed via UniFFI to Kotlin/Swift.
//!
//! These mirror the JS types from mobile-claw (auth-store, cron-db-access, etc.)
//! and the openfang message types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration for initializing the native agent handle.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct InitConfig {
    /// Path to the SQLite database.
    pub db_path: String,
    /// Path to the workspace root.
    pub workspace_path: String,
    /// Path to auth-profiles.json.
    pub auth_profiles_path: String,
}

/// Parameters for sending a message.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SendMessageParams {
    pub prompt: String,
    pub session_key: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub system_prompt: String,
    pub max_turns: Option<u32>,
    /// JSON-encoded list of allowed tool names. Empty = all tools.
    pub allowed_tools_json: Option<String>,
}

/// Auth token result.
#[derive(Debug, Clone, uniffi::Record)]
pub struct AuthTokenResult {
    pub api_key: Option<String>,
    pub is_oauth: bool,
}

/// Auth status result.
#[derive(Debug, Clone, uniffi::Record)]
pub struct AuthStatusResult {
    pub has_key: bool,
    pub masked: String,
    pub provider: String,
}

/// Token usage from an agent turn.
#[derive(Debug, Clone, Default, Serialize, Deserialize, uniffi::Record)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// Buffered event emitted while no foreground callback is attached.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct PendingEvent {
    pub id: i64,
    pub event_type: String,
    pub payload_json: String,
    pub created_at: i64,
}

// ── Internal types (not UniFFI-exported, used by agent loop) ──

/// Role in a conversation message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Content block in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "server_tool_use")]
    ServerToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult {
        tool_use_id: String,
        content: serde_json::Value, // encrypted — preserve for multi-turn citations
    },
}

/// Message content — simple text or structured blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// A conversation message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

impl Message {
    pub fn user(text: &str) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(text.to_string()),
        }
    }

    pub fn assistant_text(text: &str) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Blocks(vec![ContentBlock::Text {
                text: text.to_string(),
            }]),
        }
    }

    pub fn assistant_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }

    pub fn tool_result(tool_use_id: &str, content: &str, is_error: bool) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.to_string(),
                content: content.to_string(),
                is_error,
            }]),
        }
    }

    /// Extract text from assistant message content blocks.
    pub fn text(&self) -> String {
        match &self.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
}

/// Tool definition for the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub webview_only: bool,
}

/// A tool call from the LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// In-memory state for the current interactive session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_key: String,
    pub agent_id: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub system_prompt: String,
    pub max_turns: Option<u32>,
    pub allowed_tools_json: Option<String>,
    pub messages: Vec<Message>,
}

impl SessionState {
    pub fn to_params(&self, prompt: String) -> SendMessageParams {
        SendMessageParams {
            prompt,
            session_key: self.session_key.clone(),
            model: self.model.clone(),
            provider: self.provider.clone(),
            system_prompt: self.system_prompt.clone(),
            max_turns: self.max_turns,
            allowed_tools_json: self.allowed_tools_json.clone(),
        }
    }
}

/// Response payload for a pending tool approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub tool_call_id: String,
    pub approved: bool,
    pub reason: Option<String>,
}

/// Result payload returned from a JS-hosted MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub result_json: String,
    pub is_error: bool,
}

/// Per-skill execution state tracked by the native handle.
#[derive(Debug, Clone)]
pub struct SkillSession {
    pub session_key: String,
    pub abort_flag: Arc<Mutex<bool>>,
}

pub type SkillSessions = HashMap<String, SkillSession>;
