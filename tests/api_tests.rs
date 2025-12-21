use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Re-create a minimal client for testing (since GrooveClient isn't exported as pub)
// In a real scenario, we'd export the client module as pub
mod helpers {
    use serde_json::{json, Value};
    use std::time::Duration;

    pub struct TestClient {
        client: reqwest::Client,
        endpoint: String,
        token: String,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct GraphQLResponse<T> {
        pub data: Option<T>,
        pub errors: Option<Vec<GraphQLError>>,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct GraphQLError {
        pub message: String,
    }

    impl TestClient {
        pub fn new(token: &str, endpoint: &str) -> Self {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap();

            Self {
                client,
                endpoint: endpoint.to_string(),
                token: token.to_string(),
            }
        }

        pub async fn execute<T: for<'de> serde::Deserialize<'de>>(
            &self,
            query: &str,
            variables: Option<Value>,
        ) -> Result<T, String> {
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
                .await
                .map_err(|e| e.to_string())?;

            let status = response.status();

            if status == 429 {
                return Err("Rate limited".to_string());
            }

            if status == 401 {
                return Err("Unauthorized".to_string());
            }

            let response_body: GraphQLResponse<T> =
                response.json().await.map_err(|e| e.to_string())?;

            if let Some(errors) = response_body.errors {
                let msg = errors
                    .iter()
                    .map(|e| e.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(format!("GraphQL error: {}", msg));
            }

            response_body
                .data
                .ok_or_else(|| "No data in response".to_string())
        }
    }
}

use helpers::TestClient;

#[tokio::test]
async fn test_me_query() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "me": {
                    "id": "agent-123",
                    "email": "test@example.com",
                    "name": "Test User",
                    "role": "admin"
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = TestClient::new("test-token", &mock_server.uri());

    #[derive(Debug, serde::Deserialize)]
    struct MeResponse {
        me: Me,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Me {
        id: String,
        email: String,
        name: Option<String>,
        role: Option<String>,
    }

    let query = r#"query { me { id email name role } }"#;
    let result: MeResponse = client.execute(query, None).await.unwrap();

    assert_eq!(result.me.email, "test@example.com");
    assert_eq!(result.me.name, Some("Test User".to_string()));
    assert_eq!(result.me.role, Some("admin".to_string()));
}

#[tokio::test]
async fn test_auth_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let client = TestClient::new("invalid-token", &mock_server.uri());

    #[derive(Debug, serde::Deserialize)]
    struct DummyResponse {}

    let query = r#"query { me { id } }"#;
    let result: Result<DummyResponse, String> = client.execute(query, None).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unauthorized"));
}

#[tokio::test]
async fn test_rate_limiting() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(429).insert_header("Retry-After", "60"),
        )
        .mount(&mock_server)
        .await;

    let client = TestClient::new("test-token", &mock_server.uri());

    #[derive(Debug, serde::Deserialize)]
    struct DummyResponse {}

    let query = r#"query { me { id } }"#;
    let result: Result<DummyResponse, String> = client.execute(query, None).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Rate limited"));
}

#[tokio::test]
async fn test_graphql_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": null,
            "errors": [
                { "message": "Conversation not found" }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = TestClient::new("test-token", &mock_server.uri());

    #[derive(Debug, serde::Deserialize)]
    struct DummyResponse {}

    let query = r#"query { conversation(number: 999) { id } }"#;
    let result: Result<DummyResponse, String> = client.execute(query, None).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("GraphQL error"));
    assert!(err.contains("Conversation not found"));
}

#[tokio::test]
async fn test_conversations_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "conversations": {
                    "nodes": [
                        {
                            "id": "conv-1",
                            "number": 1,
                            "subject": "Test Subject",
                            "state": "OPENED",
                            "createdAt": "2024-01-01T00:00:00Z",
                            "updatedAt": "2024-01-01T12:00:00Z",
                            "snoozedUntil": null,
                            "messagesCount": 5,
                            "assigned": null,
                            "contact": {
                                "id": "contact-1",
                                "email": "customer@example.com",
                                "name": "Customer"
                            },
                            "channel": {
                                "id": "channel-1",
                                "name": "Email"
                            },
                            "tags": []
                        }
                    ],
                    "pageInfo": {
                        "hasNextPage": false,
                        "endCursor": null
                    },
                    "totalCount": 1
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = TestClient::new("test-token", &mock_server.uri());

    #[derive(Debug, serde::Deserialize)]
    struct ConversationsResponse {
        conversations: Conversations,
    }

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Conversations {
        nodes: Vec<Conversation>,
        page_info: PageInfo,
        total_count: i32,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Conversation {
        id: String,
        number: i64,
        subject: Option<String>,
    }

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PageInfo {
        has_next_page: bool,
        end_cursor: Option<String>,
    }

    let query = r#"query { conversations(first: 25) { nodes { id number subject } pageInfo { hasNextPage endCursor } totalCount } }"#;
    let result: ConversationsResponse = client.execute(query, None).await.unwrap();

    assert_eq!(result.conversations.nodes.len(), 1);
    assert_eq!(result.conversations.nodes[0].number, 1);
    assert_eq!(
        result.conversations.nodes[0].subject,
        Some("Test Subject".to_string())
    );
    assert_eq!(result.conversations.total_count, 1);
}

#[tokio::test]
async fn test_mutation_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "conversationClose": {
                    "errors": []
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = TestClient::new("test-token", &mock_server.uri());

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CloseResponse {
        conversation_close: MutationResult,
    }

    #[derive(Debug, serde::Deserialize)]
    struct MutationResult {
        errors: Vec<MutationError>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct MutationError {
        message: String,
    }

    let query = r#"mutation { conversationClose(input: { conversationId: "conv-1" }) { errors { message } } }"#;
    let result: CloseResponse = client.execute(query, None).await.unwrap();

    assert!(result.conversation_close.errors.is_empty());
}

#[tokio::test]
async fn test_mutation_with_errors() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "conversationClose": {
                    "errors": [
                        { "message": "Conversation is already closed" }
                    ]
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = TestClient::new("test-token", &mock_server.uri());

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CloseResponse {
        conversation_close: MutationResult,
    }

    #[derive(Debug, serde::Deserialize)]
    struct MutationResult {
        errors: Vec<MutationError>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct MutationError {
        message: String,
    }

    let query = r#"mutation { conversationClose(input: { conversationId: "conv-1" }) { errors { message } } }"#;
    let result: CloseResponse = client.execute(query, None).await.unwrap();

    assert_eq!(result.conversation_close.errors.len(), 1);
    assert_eq!(
        result.conversation_close.errors[0].message,
        "Conversation is already closed"
    );
}
