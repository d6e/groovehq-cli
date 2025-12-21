use crate::error::{GrooveError, Result};
use crate::types::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://api.groovehq.com/v2/graphql";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize)]
struct MutationResult {
    errors: Vec<MutationError>,
}

#[derive(Debug, Deserialize)]
struct MutationError {
    message: String,
}

impl MutationResult {
    fn into_result(self) -> Result<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            let msg = self
                .errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            Err(GrooveError::GraphQL(msg))
        }
    }
}

pub struct GrooveClient {
    client: Client,
    endpoint: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

impl GrooveClient {
    pub fn new(token: &str, endpoint: Option<&str>) -> Result<Self> {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(GrooveError::Network)?;

        Ok(Self {
            client,
            endpoint: endpoint.unwrap_or(DEFAULT_ENDPOINT).to_string(),
            token: token.to_string(),
        })
    }

    async fn execute<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: Option<Value>,
    ) -> Result<T> {
        let body = json!({
            "query": query,
            "variables": variables.unwrap_or(json!({}))
        });

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();

        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            return Err(GrooveError::RateLimited { retry_after });
        }

        if status == 401 {
            return Err(GrooveError::AuthError("Invalid or expired token".into()));
        }

        let response_body: GraphQLResponse<T> = response.json().await?;

        if let Some(errors) = response_body.errors {
            let msg = errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(GrooveError::GraphQL(msg));
        }

        response_body
            .data
            .ok_or_else(|| GrooveError::GraphQL("No data in response".into()))
    }

    pub async fn me(&self) -> Result<CurrentAgent> {
        #[derive(Deserialize)]
        struct Response {
            me: CurrentAgent,
        }

        let query = r#"
            query {
                me {
                    id
                    email
                    name
                    role
                }
            }
        "#;

        let response: Response = self.execute(query, None).await?;
        Ok(response.me)
    }

    pub async fn conversations(
        &self,
        first: Option<u32>,
        after: Option<String>,
        state: Option<&str>,
        folder_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<ConversationsResponse> {
        #[derive(Deserialize)]
        struct Response {
            conversations: ConversationsResponse,
        }

        let query = r#"
            query Conversations($first: Int, $after: String, $filter: ConversationFilter) {
                conversations(first: $first, after: $after, filter: $filter) {
                    nodes {
                        id
                        number
                        subject
                        state
                        createdAt
                        updatedAt
                        snoozedUntil
                        messagesCount
                        assigned {
                            ... on Agent {
                                id
                                email
                                name
                            }
                        }
                        contact {
                            id
                            email
                            name
                        }
                        channel {
                            id
                            name
                        }
                        tags {
                            id
                            name
                            color
                        }
                    }
                    pageInfo {
                        hasNextPage
                        endCursor
                    }
                    totalCount
                }
            }
        "#;

        let mut filter = json!({});
        if let Some(s) = state {
            filter["state"] = json!(s.to_uppercase());
        }
        if let Some(f) = folder_id {
            filter["folderId"] = json!(f);
        }
        if let Some(q) = search {
            filter["keywords"] = json!(q);
        }

        let variables = json!({
            "first": first.unwrap_or(25),
            "after": after,
            "filter": if filter.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                Value::Null
            } else {
                filter
            }
        });

        let response: Response = self.execute(query, Some(variables)).await?;
        Ok(response.conversations)
    }

    pub async fn conversation(&self, number: i64) -> Result<Conversation> {
        #[derive(Deserialize)]
        struct Response {
            conversation: Option<Conversation>,
        }

        let query = r#"
            query Conversation($number: Int!) {
                conversation(number: $number) {
                    id
                    number
                    subject
                    state
                    createdAt
                    updatedAt
                    snoozedUntil
                    messagesCount
                    assigned {
                        ... on Agent {
                            id
                            email
                            name
                        }
                    }
                    contact {
                        id
                        email
                        name
                    }
                    channel {
                        id
                        name
                    }
                    tags {
                        id
                        name
                        color
                    }
                }
            }
        "#;

        let variables = json!({ "number": number });
        let response: Response = self.execute(query, Some(variables)).await?;
        response
            .conversation
            .ok_or(GrooveError::ConversationNotFound(number))
    }

    pub async fn messages(&self, conversation_id: &str, first: Option<i32>) -> Result<Vec<Message>> {
        #[derive(Deserialize)]
        struct Response {
            node: Option<ConversationWithMessages>,
        }

        #[derive(Deserialize)]
        struct ConversationWithMessages {
            messages: MessagesConnection,
        }

        #[derive(Deserialize)]
        struct MessagesConnection {
            nodes: Vec<Message>,
        }

        let query = r#"
            query Messages($id: ID!, $first: Int) {
                node(id: $id) {
                    ... on Conversation {
                        messages(first: $first) {
                            nodes {
                                id
                                createdAt
                                bodyText
                                bodyHtml
                                author {
                                    __typename
                                    ... on Agent {
                                        id
                                        email
                                        name
                                    }
                                    ... on Contact {
                                        id
                                        email
                                        name
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;

        let variables = json!({
            "id": conversation_id,
            "first": first.unwrap_or(50)
        });

        let response: Response = self.execute(query, Some(variables)).await?;
        Ok(response
            .node
            .map(|n| n.messages.nodes)
            .unwrap_or_default())
    }

    pub async fn folders(&self) -> Result<Vec<Folder>> {
        #[derive(Deserialize)]
        struct Response {
            folders: FoldersConnection,
        }

        #[derive(Deserialize)]
        struct FoldersConnection {
            nodes: Vec<Folder>,
        }

        let query = r#"
            query {
                folders(first: 100) {
                    nodes {
                        id
                        name
                        count
                    }
                }
            }
        "#;

        let response: Response = self.execute(query, None).await?;
        Ok(response.folders.nodes)
    }

    pub async fn tags(&self) -> Result<Vec<Tag>> {
        #[derive(Deserialize)]
        struct Response {
            tags: TagsConnection,
        }

        #[derive(Deserialize)]
        struct TagsConnection {
            nodes: Vec<Tag>,
        }

        let query = r#"
            query {
                tags(first: 100) {
                    nodes {
                        id
                        name
                        color
                    }
                }
            }
        "#;

        let response: Response = self.execute(query, None).await?;
        Ok(response.tags.nodes)
    }

    pub async fn canned_replies(&self) -> Result<Vec<CannedReply>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            canned_replies: CannedRepliesConnection,
        }

        #[derive(Deserialize)]
        struct CannedRepliesConnection {
            nodes: Vec<CannedReply>,
        }

        let query = r#"
            query {
                cannedReplies(first: 100) {
                    nodes {
                        id
                        name
                        subject
                        body
                    }
                }
            }
        "#;

        let response: Response = self.execute(query, None).await?;
        Ok(response.canned_replies.nodes)
    }

    pub async fn reply(&self, conversation_id: &str, body: &str) -> Result<()> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            conversation_reply: MutationResult,
        }

        let query = r#"
            mutation Reply($input: ConversationReplyInput!) {
                conversationReply(input: $input) {
                    errors {
                        message
                    }
                }
            }
        "#;

        let variables = json!({
            "input": {
                "conversationId": conversation_id,
                "body": body
            }
        });

        let response: Response = self.execute(query, Some(variables)).await?;
        response.conversation_reply.into_result()
    }

    pub async fn close(&self, conversation_id: &str) -> Result<()> {
        self.update_state(conversation_id, "conversationClose").await
    }

    pub async fn open(&self, conversation_id: &str) -> Result<()> {
        self.update_state(conversation_id, "conversationOpen").await
    }

    async fn update_state(&self, conversation_id: &str, mutation: &str) -> Result<()> {
        let query = format!(
            r#"
            mutation UpdateState($input: ConversationStateInput!) {{
                {}(input: $input) {{
                    errors {{
                        message
                    }}
                }}
            }}
        "#,
            mutation
        );

        #[derive(Deserialize)]
        struct Response {
            #[serde(flatten)]
            result: std::collections::HashMap<String, MutationResult>,
        }

        let variables = json!({
            "input": {
                "conversationId": conversation_id
            }
        });

        let response: Response = self.execute(&query, Some(variables)).await?;
        for (_, result) in response.result {
            result.into_result()?;
        }
        Ok(())
    }

    pub async fn snooze(&self, conversation_id: &str, until: &str) -> Result<()> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            conversation_snooze: MutationResult,
        }

        let query = r#"
            mutation Snooze($input: ConversationSnoozeInput!) {
                conversationSnooze(input: $input) {
                    errors {
                        message
                    }
                }
            }
        "#;

        let variables = json!({
            "input": {
                "conversationId": conversation_id,
                "snoozedUntil": until
            }
        });

        let response: Response = self.execute(query, Some(variables)).await?;
        response.conversation_snooze.into_result()
    }

    pub async fn assign(&self, conversation_id: &str, agent_id: &str) -> Result<()> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            conversation_assign: MutationResult,
        }

        let query = r#"
            mutation Assign($input: ConversationAssignInput!) {
                conversationAssign(input: $input) {
                    errors {
                        message
                    }
                }
            }
        "#;

        let variables = json!({
            "input": {
                "conversationId": conversation_id,
                "assigneeId": agent_id
            }
        });

        let response: Response = self.execute(query, Some(variables)).await?;
        response.conversation_assign.into_result()
    }

    pub async fn add_note(&self, conversation_id: &str, body: &str) -> Result<()> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            conversation_add_note: MutationResult,
        }

        let query = r#"
            mutation AddNote($input: ConversationAddNoteInput!) {
                conversationAddNote(input: $input) {
                    errors {
                        message
                    }
                }
            }
        "#;

        let variables = json!({
            "input": {
                "conversationId": conversation_id,
                "body": body
            }
        });

        let response: Response = self.execute(query, Some(variables)).await?;
        response.conversation_add_note.into_result()
    }

    pub async fn tag(&self, conversation_id: &str, tag_ids: Vec<String>) -> Result<()> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            conversation_tag: MutationResult,
        }

        let query = r#"
            mutation Tag($input: ConversationTagInput!) {
                conversationTag(input: $input) {
                    errors {
                        message
                    }
                }
            }
        "#;

        let variables = json!({
            "input": {
                "conversationId": conversation_id,
                "tagIds": tag_ids
            }
        });

        let response: Response = self.execute(query, Some(variables)).await?;
        response.conversation_tag.into_result()
    }

    pub async fn agents(&self) -> Result<Vec<Agent>> {
        #[derive(Deserialize)]
        struct Response {
            agents: AgentsConnection,
        }

        #[derive(Deserialize)]
        struct AgentsConnection {
            nodes: Vec<Agent>,
        }

        let query = r#"
            query {
                agents(first: 100) {
                    nodes {
                        id
                        email
                        name
                    }
                }
            }
        "#;

        let response: Response = self.execute(query, None).await?;
        Ok(response.agents.nodes)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationsResponse {
    pub nodes: Vec<Conversation>,
    pub page_info: PageInfo,
    pub total_count: i32,
}
