use crate::api::ConversationsResponse;
use crate::cli::OutputFormat;
use crate::types::*;
use chrono::{DateTime, Utc};
use colored::Colorize;
use tabled::settings::Style;
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct ConversationRow {
    #[tabled(rename = "#")]
    number: i64,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Subject")]
    subject: String,
    #[tabled(rename = "From")]
    from: String,
    #[tabled(rename = "Updated")]
    updated: String,
}

impl ConversationRow {
    fn from_conversation(conv: &Conversation) -> Self {
        let status = format_state(&conv.state);
        let subject = truncate(conv.subject.as_deref().unwrap_or("(no subject)"), 40);
        let contact = conv
            .contact
            .as_ref()
            .and_then(|c| c.email.as_deref().or(c.name.as_deref()))
            .unwrap_or("unknown");
        let updated = format_relative_time(&conv.updated_at);

        Self {
            number: conv.number,
            status: format!("{}", status.color(state_color_str(&conv.state))),
            subject,
            from: truncate(contact, 25),
            updated,
        }
    }
}

#[derive(Tabled)]
struct FolderRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "ID")]
    id: String,
}

impl From<&Folder> for FolderRow {
    fn from(folder: &Folder) -> Self {
        Self {
            name: folder.name.clone(),
            id: folder.id.clone(),
        }
    }
}

#[derive(Tabled)]
struct TagRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Color")]
    color: String,
    #[tabled(rename = "ID")]
    id: String,
}

impl From<&Tag> for TagRow {
    fn from(tag: &Tag) -> Self {
        Self {
            name: tag.name.clone(),
            color: tag.color.as_deref().unwrap_or("-").to_string(),
            id: tag.id.clone(),
        }
    }
}

#[derive(Tabled)]
struct CannedReplyRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Subject")]
    subject: String,
    #[tabled(rename = "ID")]
    id: String,
}

impl From<&CannedReply> for CannedReplyRow {
    fn from(reply: &CannedReply) -> Self {
        Self {
            name: reply.name.clone(),
            subject: reply.subject.as_deref().unwrap_or("-").to_string(),
            id: reply.id.clone(),
        }
    }
}

pub fn format_conversations(response: &ConversationsResponse, format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(response).expect("serialization should not fail")
            );
        }
        OutputFormat::Compact => {
            for conv in &response.nodes {
                let status = format!("[{}]", conv.state);
                let subject = conv.subject.as_deref().unwrap_or("(no subject)");
                let contact = conv
                    .contact
                    .as_ref()
                    .and_then(|c| c.email.as_deref())
                    .unwrap_or("unknown");
                println!("#{} {} {} - {}", conv.number, status, subject, contact);
            }
        }
        OutputFormat::Table => {
            let rows: Vec<ConversationRow> = response
                .nodes
                .iter()
                .map(ConversationRow::from_conversation)
                .collect();
            let table = Table::new(rows).with(Style::rounded()).to_string();

            println!("{table}");
            println!(
                "\nShowing {} of {} conversations",
                response.nodes.len(),
                response.total_count
            );

            if response.page_info.has_next_page {
                if let Some(cursor) = &response.page_info.end_cursor {
                    println!("Next page: --after {}", cursor);
                }
            }
        }
    }
}

pub fn format_conversation_detail(conv: &Conversation, messages: &[Message], full: bool) {
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "{} #{}",
        "Conversation".bold(),
        conv.number.to_string().bold()
    );
    println!("{}", "─".repeat(60).dimmed());

    if let Some(subject) = &conv.subject {
        println!("{}: {}", "Subject".dimmed(), subject);
    }

    println!(
        "{}: {}",
        "Status".dimmed(),
        format_state(&conv.state).color(state_color_str(&conv.state))
    );

    if let Some(contact) = &conv.contact {
        let name = contact.name.as_deref().unwrap_or("");
        let email = contact.email.as_deref().unwrap_or("unknown");
        if name.is_empty() {
            println!("{}: {}", "From".dimmed(), email);
        } else {
            println!("{}: {} <{}>", "From".dimmed(), name, email);
        }
    }

    if let Some(agent) = &conv.assigned {
        let name = agent.name.as_deref().unwrap_or(&agent.email);
        println!("{}: {}", "Assigned".dimmed(), name);
    } else {
        println!("{}: {}", "Assigned".dimmed(), "unassigned".yellow());
    }

    if !conv.tags.is_empty() {
        let tags: Vec<_> = conv.tags.iter().map(|t| t.name.as_str()).collect();
        println!("{}: {}", "Tags".dimmed(), tags.join(", "));
    }

    println!(
        "{}: {}",
        "Created".dimmed(),
        conv.created_at.format("%Y-%m-%d %H:%M")
    );

    println!("{}", "─".repeat(60).dimmed());
    println!();

    for msg in messages {
        print_message(msg, full);
    }
}

fn print_message(msg: &Message, full: bool) {
    let author_name = msg
        .author
        .as_ref()
        .and_then(|a| a.name.as_deref().or(a.email.as_deref()))
        .unwrap_or("Unknown");

    let author_type = msg
        .author
        .as_ref()
        .and_then(|a| a.typename.as_deref())
        .unwrap_or("Unknown");

    let time = msg.created_at.format("%b %d, %H:%M");

    let label = match author_type {
        "Agent" => format!("[Agent] {}", author_name).cyan(),
        "Contact" => format!("[Customer] {}", author_name).green(),
        _ => format!("[{}] {}", author_type, author_name).normal(),
    };

    println!("{} • {}", label, time.to_string().dimmed());

    if let Some(body) = &msg.body_text {
        let text = if full {
            body.clone()
        } else {
            truncate_lines(body, 10)
        };
        println!("{}\n", text);
    }
}

pub fn format_folders(folders: &[Folder], format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(folders).expect("serialization should not fail")
            );
        }
        OutputFormat::Compact => {
            for folder in folders {
                println!("{}", folder.name);
            }
        }
        OutputFormat::Table => {
            let rows: Vec<FolderRow> = folders.iter().map(FolderRow::from).collect();
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{table}");
        }
    }
}

pub fn format_tags(tags: &[Tag], format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(tags).expect("serialization should not fail")
            );
        }
        OutputFormat::Compact => {
            for tag in tags {
                println!("{}", tag.name);
            }
        }
        OutputFormat::Table => {
            let rows: Vec<TagRow> = tags.iter().map(TagRow::from).collect();
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{table}");
        }
    }
}

pub fn format_canned_replies(replies: &[CannedReply], format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(replies).expect("serialization should not fail")
            );
        }
        OutputFormat::Compact => {
            for reply in replies {
                println!("{}", reply.name);
            }
        }
        OutputFormat::Table => {
            let rows: Vec<CannedReplyRow> = replies.iter().map(CannedReplyRow::from).collect();
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{table}");
        }
    }
}

pub fn format_canned_reply(reply: &CannedReply) {
    println!("{}: {}", "Name".dimmed(), reply.name);
    if let Some(subject) = &reply.subject {
        println!("{}: {}", "Subject".dimmed(), subject);
    }
    println!("{}", "─".repeat(40).dimmed());
    if let Some(body) = &reply.body {
        println!("{}", body);
    }
}

pub fn format_agent(agent: &CurrentAgent, format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(agent).expect("serialization should not fail")
            );
        }
        _ => {
            println!(
                "{}: {}",
                "Name".dimmed(),
                agent.name.as_deref().unwrap_or("-")
            );
            println!("{}: {}", "Email".dimmed(), agent.email);
            if let Some(role) = &agent.role {
                println!("{}: {}", "Role".dimmed(), role);
            }
            println!("{}: {}", "ID".dimmed(), agent.id);
        }
    }
}

fn format_state(state: &ConversationState) -> String {
    match state {
        ConversationState::Unread => "unread".to_string(),
        ConversationState::Opened => "open".to_string(),
        ConversationState::Closed => "closed".to_string(),
        ConversationState::Snoozed => "snoozed".to_string(),
        ConversationState::Spam => "spam".to_string(),
        ConversationState::Deleted => "deleted".to_string(),
    }
}

fn state_color_str(state: &ConversationState) -> &'static str {
    match state {
        ConversationState::Unread => "yellow",
        ConversationState::Opened => "green",
        ConversationState::Closed => "white",
        ConversationState::Snoozed => "blue",
        ConversationState::Spam => "red",
        ConversationState::Deleted => "white",
    }
}

fn format_relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_minutes() < 1 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{}d ago", duration.num_days())
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

fn truncate_lines(s: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= max_lines {
        s.to_string()
    } else {
        let truncated: Vec<&str> = lines.into_iter().take(max_lines).collect();
        format!(
            "{}\n  [... truncated, use --full to see all]",
            truncated.join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        let result = truncate("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        let result = truncate("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("hello world", 8);
        assert_eq!(result, "hello w…");
    }

    #[test]
    fn test_truncate_unicode() {
        let result = truncate("héllo wörld", 8);
        assert_eq!(result, "héllo w…");
    }

    #[test]
    fn test_truncate_lines_short() {
        let input = "line1\nline2\nline3";
        let result = truncate_lines(input, 5);
        assert_eq!(result, input);
    }

    #[test]
    fn test_truncate_lines_exact() {
        let input = "line1\nline2\nline3";
        let result = truncate_lines(input, 3);
        assert_eq!(result, input);
    }

    #[test]
    fn test_truncate_lines_truncated() {
        let input = "line1\nline2\nline3\nline4\nline5";
        let result = truncate_lines(input, 2);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(!result.contains("line3"));
        assert!(result.contains("truncated"));
    }

    #[test]
    fn test_format_state_all_variants() {
        assert_eq!(format_state(&ConversationState::Unread), "unread");
        assert_eq!(format_state(&ConversationState::Opened), "open");
        assert_eq!(format_state(&ConversationState::Closed), "closed");
        assert_eq!(format_state(&ConversationState::Snoozed), "snoozed");
        assert_eq!(format_state(&ConversationState::Spam), "spam");
        assert_eq!(format_state(&ConversationState::Deleted), "deleted");
    }

    #[test]
    fn test_format_relative_time_just_now() {
        let now = Utc::now();
        let result = format_relative_time(&now);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_format_relative_time_minutes() {
        let time = Utc::now() - chrono::Duration::minutes(30);
        let result = format_relative_time(&time);
        assert!(result.contains("m ago"));
    }

    #[test]
    fn test_format_relative_time_hours() {
        let time = Utc::now() - chrono::Duration::hours(5);
        let result = format_relative_time(&time);
        assert!(result.contains("h ago"));
    }

    #[test]
    fn test_format_relative_time_days() {
        let time = Utc::now() - chrono::Duration::days(3);
        let result = format_relative_time(&time);
        assert!(result.contains("d ago"));
    }

    #[test]
    fn test_format_relative_time_old() {
        let time = Utc::now() - chrono::Duration::days(30);
        let result = format_relative_time(&time);
        // Should show date format YYYY-MM-DD
        assert!(result.contains("-"));
        assert!(!result.contains("ago"));
    }
}
