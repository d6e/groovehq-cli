use groovehq_cli::api::GrooveClient;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

    let client = GrooveClient::new("test-token", Some(&mock_server.uri())).unwrap();
    let result = client.me().await.unwrap();

    assert_eq!(result.email, "test@example.com");
    assert_eq!(result.name, Some("Test User".to_string()));
    assert_eq!(result.role, Some("admin".to_string()));
}

#[tokio::test]
async fn test_auth_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let client = GrooveClient::new("invalid-token", Some(&mock_server.uri())).unwrap();
    let result = client.me().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Authentication failed"));
}

#[tokio::test]
async fn test_rate_limiting() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "60"))
        .mount(&mock_server)
        .await;

    let client = GrooveClient::new("test-token", Some(&mock_server.uri())).unwrap();
    let result = client.me().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Rate limited"));
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

    let client = GrooveClient::new("test-token", Some(&mock_server.uri())).unwrap();
    let result = client.conversation(999).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Conversation not found"));
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

    let client = GrooveClient::new("test-token", Some(&mock_server.uri())).unwrap();
    let result = client
        .conversations(Some(25), None, None, None, None)
        .await
        .unwrap();

    assert_eq!(result.nodes.len(), 1);
    assert_eq!(result.nodes[0].number, 1);
    assert_eq!(result.nodes[0].subject, Some("Test Subject".to_string()));
    assert_eq!(result.total_count, 1);
}

#[tokio::test]
async fn test_folders_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "folders": {
                    "nodes": [
                        {
                            "id": "folder-1",
                            "name": "Inbox",
                            "count": 42
                        },
                        {
                            "id": "folder-2",
                            "name": "Archive",
                            "count": 100
                        }
                    ]
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = GrooveClient::new("test-token", Some(&mock_server.uri())).unwrap();
    let result = client.folders().await.unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Inbox");
    assert_eq!(result[0].count, Some(42));
}

#[tokio::test]
async fn test_tags_list() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "tags": {
                    "nodes": [
                        {
                            "id": "tag-1",
                            "name": "urgent",
                            "color": "#ff0000"
                        }
                    ]
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = GrooveClient::new("test-token", Some(&mock_server.uri())).unwrap();
    let result = client.tags().await.unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "urgent");
    assert_eq!(result[0].color, Some("#ff0000".to_string()));
}

#[tokio::test]
async fn test_close_conversation() {
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

    let client = GrooveClient::new("test-token", Some(&mock_server.uri())).unwrap();
    let result = client.close("conv-1").await;

    assert!(result.is_ok());
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

    let client = GrooveClient::new("test-token", Some(&mock_server.uri())).unwrap();
    let result = client.close("conv-1").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("already closed"));
}
