use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// Wrapper for the Assignment type that contains an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub agent: Option<Agent>,
}

/// Wrapper for connection types that have nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagConnection {
    #[serde(default)]
    pub nodes: Vec<Tag>,
}

fn deserialize_assigned<'de, D>(deserializer: D) -> Result<Option<Agent>, D::Error>
where
    D: Deserializer<'de>,
{
    let assignment: Option<Assignment> = Option::deserialize(deserializer)?;
    Ok(assignment.and_then(|a| a.agent))
}

fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<Tag>, D::Error>
where
    D: Deserializer<'de>,
{
    let connection: Option<TagConnection> = Option::deserialize(deserializer)?;
    Ok(connection.map(|c| c.nodes).unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub number: i64,
    pub subject: Option<String>,
    pub state: ConversationState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, deserialize_with = "deserialize_assigned")]
    pub assigned: Option<Agent>,
    #[serde(default)]
    pub channel: Option<Channel>,
    #[serde(default)]
    pub contact: Option<Contact>,
    #[serde(default, deserialize_with = "deserialize_tags")]
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversationState {
    Unread,
    Opened,
    Closed,
    Snoozed,
    Spam,
    Deleted,
}

impl std::fmt::Display for ConversationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversationState::Unread => write!(f, "unread"),
            ConversationState::Opened => write!(f, "open"),
            ConversationState::Closed => write!(f, "closed"),
            ConversationState::Snoozed => write!(f, "snoozed"),
            ConversationState::Spam => write!(f, "spam"),
            ConversationState::Deleted => write!(f, "deleted"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    #[serde(default)]
    pub author: Option<MessageAuthor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageAuthor {
    #[serde(rename = "__typename")]
    pub typename: Option<String>,
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CannedReply {
    pub id: String,
    pub name: String,
    pub subject: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentAgent {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}
