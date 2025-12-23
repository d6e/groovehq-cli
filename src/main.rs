use anyhow::Context;
use chrono::{Duration, Utc};
use clap::Parser;
use std::io::{self, IsTerminal, Read};

use groovehq_cli::api::{GrooveClient, MAX_ITEMS_PER_PAGE};
use groovehq_cli::cli::{
    self, print_completions, CannedRepliesAction, Cli, Commands, ConfigAction, ConversationAction,
    FolderAction, OutputFormat, TagAction,
};
use groovehq_cli::config::{self, Config};
use groovehq_cli::error;

const DEFAULT_CONVERSATION_LIMIT: u32 = 25;
const DEFAULT_MESSAGE_LIMIT: i32 = 50;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err}");

        // Show error chain if verbose flag was passed
        if std::env::args().any(|arg| arg == "--verbose" || arg == "-v") {
            let mut source = err.source();
            while let Some(cause) = source {
                eprintln!("Caused by: {cause}");
                source = cause.source();
            }
        }

        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load().context("Failed to load configuration")?;

    // Resolve format: CLI flag > config default > "table"
    let format = cli.format.unwrap_or_else(|| {
        config
            .defaults
            .format
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(OutputFormat::Table)
    });

    match &cli.command {
        Commands::Config { action } => handle_config(&action, &config, cli.quiet)?,
        Commands::Completions { shell } => {
            print_completions(shell.clone());
        }
        _ => {
            let token = config::resolve_token(cli.token.as_deref(), &config)?;
            let client = GrooveClient::new(&token, config.api_endpoint.as_deref())?;
            handle_command(&cli.command, &client, &format, &config, cli.quiet).await?;
        }
    }

    Ok(())
}

fn handle_config(action: &ConfigAction, config: &Config, quiet: bool) -> anyhow::Result<()> {
    match action {
        ConfigAction::Show => {
            if let Some(token) = &config.api_token {
                let masked = if token.len() > 8 {
                    format!("{}...{}", &token[..4], &token[token.len() - 4..])
                } else {
                    "********".to_string()
                };
                println!("api_token: {}", masked);
            } else {
                println!("api_token: (not set)");
            }
            if let Some(endpoint) = &config.api_endpoint {
                println!("api_endpoint: {}", endpoint);
            }
        }
        ConfigAction::SetToken { token } => {
            let mut config = config.clone();
            config.set_token(token.clone())?;
            if !quiet {
                println!("Token saved successfully");
            }
        }
        ConfigAction::Path => {
            if let Some(path) = Config::path() {
                println!("{}", path.display());
            } else {
                println!("Could not determine config path");
            }
        }
    }
    Ok(())
}

async fn handle_command(
    command: &Commands,
    client: &GrooveClient,
    format: &OutputFormat,
    config: &Config,
    quiet: bool,
) -> anyhow::Result<()> {
    match command {
        Commands::Me => {
            let agent = client.me().await?;
            cli::format_agent(&agent, format);
        }

        Commands::Conversation { action } => {
            handle_conversation(action, client, format, config, quiet).await?;
        }

        Commands::Folder { action } => {
            handle_folder(action, client, format).await?;
        }

        Commands::Tag { action } => {
            handle_tag(action, client, format).await?;
        }

        Commands::CannedReplies { action } => {
            handle_canned_replies(action, client, format).await?;
        }

        Commands::Config { .. } | Commands::Completions { .. } => unreachable!(),
    }

    Ok(())
}

async fn handle_conversation(
    action: &ConversationAction,
    client: &GrooveClient,
    format: &OutputFormat,
    config: &Config,
    quiet: bool,
) -> anyhow::Result<()> {
    match action {
        ConversationAction::List {
            status,
            folder,
            search,
            limit,
            after,
        } => {
            // Apply config defaults: CLI arg > config default > hardcoded default
            let limit = limit
                .or(config.defaults.limit)
                .unwrap_or(DEFAULT_CONVERSATION_LIMIT);
            let folder = folder.as_ref().or(config.defaults.folder.as_ref());
            let response = client
                .conversations(
                    Some(limit),
                    after.clone(),
                    status.as_deref(),
                    folder.map(|s| s.as_str()),
                    search.as_deref(),
                )
                .await?;
            cli::format_conversations(&response, format);
        }

        ConversationAction::View { number, full } => {
            let conv = get_conversation(client, *number).await?;
            let messages = client
                .messages(&conv.id, Some(DEFAULT_MESSAGE_LIMIT))
                .await?;
            cli::format_conversation_detail(&conv, &messages, *full);
        }

        ConversationAction::Reply {
            number,
            body,
            canned,
        } => {
            let body = if let Some(canned_name) = canned {
                let canned_replies = client.canned_replies().await?;
                let canned_reply = canned_replies
                    .iter()
                    .find(|r| r.name.eq_ignore_ascii_case(canned_name) || r.id == *canned_name)
                    .ok_or_else(|| error::GrooveError::CannedReplyNotFound(canned_name.clone()))?;

                let canned_body = canned_reply.body.clone().unwrap_or_default();
                match body {
                    Some(extra) => format!("{}\n\n{}", canned_body, extra),
                    None => canned_body,
                }
            } else {
                get_body(body.clone())?
            };

            let conv = get_conversation(client, *number).await?;
            client.reply(&conv.id, &body).await?;
            success_msg(quiet, format!("Reply sent to conversation #{}", number));
        }

        ConversationAction::Close { numbers } => {
            validate_conversation_numbers(numbers)?;
            for number in numbers {
                let conv = get_conversation(client, *number).await?;
                client.close(&conv.id).await?;
                success_msg(quiet, format!("Closed conversation #{}", number));
            }
        }

        ConversationAction::Open { numbers } => {
            validate_conversation_numbers(numbers)?;
            for number in numbers {
                let conv = get_conversation(client, *number).await?;
                client.open(&conv.id).await?;
                success_msg(quiet, format!("Opened conversation #{}", number));
            }
        }

        ConversationAction::Snooze { number, duration } => {
            let until = parse_duration(duration)?;
            let conv = get_conversation(client, *number).await?;
            client.snooze(&conv.id, &until).await?;
            success_msg(
                quiet,
                format!("Snoozed conversation #{} until {}", number, until),
            );
        }

        ConversationAction::Assign { number, agent } => {
            let conv = get_conversation(client, *number).await?;

            let agent_id = if agent == "me" {
                client.me().await?.id
            } else {
                let agents = client.agents().await?;
                agents
                    .iter()
                    .find(|a| a.email == *agent || a.name.as_deref() == Some(agent))
                    .map(|a| a.id.clone())
                    .ok_or_else(|| error::GrooveError::AgentNotFound(agent.clone()))?
            };

            client.assign(&conv.id, &agent_id).await?;
            success_msg(
                quiet,
                format!("Assigned conversation #{} to {}", number, agent),
            );
        }

        ConversationAction::Unassign { numbers } => {
            validate_conversation_numbers(numbers)?;
            for number in numbers {
                let conv = get_conversation(client, *number).await?;
                client.unassign(&conv.id).await?;
                success_msg(quiet, format!("Unassigned conversation #{}", number));
            }
        }

        ConversationAction::AddTag { number, tags } => {
            let conv = get_conversation(client, *number).await?;
            let all_tags = client.tags().await?;
            let tag_ids = resolve_tag_ids(tags, &all_tags)?;
            client.tag(&conv.id, tag_ids).await?;
            success_msg(quiet, format!("Added tags to conversation #{}", number));
        }

        ConversationAction::RemoveTag { number, tags } => {
            let conv = get_conversation(client, *number).await?;
            let all_tags = client.tags().await?;
            let tag_ids = resolve_tag_ids(tags, &all_tags)?;
            client.untag(&conv.id, tag_ids).await?;
            success_msg(quiet, format!("Removed tags from conversation #{}", number));
        }

        ConversationAction::Note { number, body } => {
            let body = get_body(body.clone())?;
            let conv = get_conversation(client, *number).await?;
            client.add_note(&conv.id, &body).await?;
            success_msg(quiet, format!("Note added to conversation #{}", number));
        }
    }

    Ok(())
}

async fn handle_folder(
    action: &FolderAction,
    client: &GrooveClient,
    format: &OutputFormat,
) -> anyhow::Result<()> {
    match action {
        FolderAction::List => {
            let folders = client.folders().await?;
            cli::format_folders(&folders, format);
            if folders.len() >= MAX_ITEMS_PER_PAGE {
                eprintln!(
                    "Warning: Results may be truncated (showing {} items)",
                    MAX_ITEMS_PER_PAGE
                );
            }
        }
    }
    Ok(())
}

async fn handle_tag(
    action: &TagAction,
    client: &GrooveClient,
    format: &OutputFormat,
) -> anyhow::Result<()> {
    match action {
        TagAction::List => {
            let tags = client.tags().await?;
            cli::format_tags(&tags, format);
            if tags.len() >= MAX_ITEMS_PER_PAGE {
                eprintln!(
                    "Warning: Results may be truncated (showing {} items)",
                    MAX_ITEMS_PER_PAGE
                );
            }
        }
    }
    Ok(())
}

async fn handle_canned_replies(
    action: &CannedRepliesAction,
    client: &GrooveClient,
    format: &OutputFormat,
) -> anyhow::Result<()> {
    match action {
        CannedRepliesAction::List => {
            let replies = client.canned_replies().await?;
            cli::format_canned_replies(&replies, format);
            if replies.len() >= MAX_ITEMS_PER_PAGE {
                eprintln!(
                    "Warning: Results may be truncated (showing {} items)",
                    MAX_ITEMS_PER_PAGE
                );
            }
        }
        CannedRepliesAction::Show { name } => {
            let replies = client.canned_replies().await?;
            let reply = replies
                .iter()
                .find(|r| r.name.eq_ignore_ascii_case(name) || r.id == *name)
                .ok_or_else(|| error::GrooveError::CannedReplyNotFound(name.clone()))?;
            cli::format_canned_reply(reply);
        }
    }
    Ok(())
}

fn validate_conversation_number(number: i64) -> anyhow::Result<()> {
    if number <= 0 {
        anyhow::bail!("Conversation number must be positive, got: {}", number);
    }
    Ok(())
}

fn validate_conversation_numbers(numbers: &[i64]) -> anyhow::Result<()> {
    for number in numbers {
        validate_conversation_number(*number)?;
    }
    Ok(())
}

async fn get_conversation(
    client: &GrooveClient,
    number: i64,
) -> anyhow::Result<groovehq_cli::types::Conversation> {
    validate_conversation_number(number)?;
    Ok(client.conversation(number).await?)
}

fn resolve_tag_ids(
    tag_names: &[String],
    all_tags: &[groovehq_cli::types::Tag],
) -> anyhow::Result<Vec<String>> {
    tag_names
        .iter()
        .map(|name| {
            all_tags
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(name))
                .map(|t| t.id.clone())
                .ok_or_else(|| anyhow::anyhow!(error::GrooveError::TagNotFound(name.clone())))
        })
        .collect()
}

fn success_msg(quiet: bool, msg: impl std::fmt::Display) {
    if !quiet {
        println!("{}", msg);
    }
}

fn get_body(body_arg: Option<String>) -> anyhow::Result<String> {
    if let Some(body) = body_arg {
        return Ok(body);
    }

    // Check if stdin has data (not a TTY)
    if io::stdin().is_terminal() {
        anyhow::bail!("No body provided. Pass as argument or pipe content via stdin");
    }

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    if buffer.trim().is_empty() {
        anyhow::bail!("Empty body provided");
    }

    Ok(buffer)
}

fn parse_duration(s: &str) -> anyhow::Result<String> {
    // If it looks like an ISO datetime (contains T or is a date like YYYY-MM-DD), return as-is
    let is_iso_date = s.contains('T')
        || (s.len() >= 10
            && s.chars().take(4).all(|c| c.is_ascii_digit())
            && s.chars().nth(4) == Some('-'));

    if is_iso_date {
        return Ok(s.to_string());
    }

    let len = s.len();
    if len < 2 {
        anyhow::bail!("Invalid duration: {}", s);
    }

    let (num_str, unit) = s.split_at(len - 1);
    let num: i64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid duration number: {}", num_str))?;

    if num <= 0 {
        anyhow::bail!("Duration must be positive, got: {}", num);
    }

    let duration = match unit {
        "m" => Duration::minutes(num),
        "h" => Duration::hours(num),
        "d" => Duration::days(num),
        "w" => Duration::weeks(num),
        _ => anyhow::bail!("Invalid duration unit: {}. Use m, h, d, or w", unit),
    };

    let until = Utc::now() + duration;
    Ok(until.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_minutes() {
        let result = parse_duration("30m").unwrap();
        // Should return a valid RFC3339 datetime
        assert!(result.contains("T"));
        assert!(chrono::DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_duration_hours() {
        let result = parse_duration("2h").unwrap();
        assert!(result.contains("T"));
        assert!(chrono::DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_duration_days() {
        let result = parse_duration("5d").unwrap();
        assert!(result.contains("T"));
        assert!(chrono::DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_duration_weeks() {
        let result = parse_duration("1w").unwrap();
        assert!(result.contains("T"));
        assert!(chrono::DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_duration_iso_passthrough() {
        let iso = "2024-12-25T10:00:00Z";
        let result = parse_duration(iso).unwrap();
        assert_eq!(result, iso);
    }

    #[test]
    fn test_parse_duration_date_passthrough() {
        let date = "2024-12-25";
        let result = parse_duration(date).unwrap();
        assert_eq!(result, date);
    }

    #[test]
    fn test_parse_duration_invalid_too_short() {
        let result = parse_duration("h");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid duration"));
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        let result = parse_duration("5x");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid duration unit"));
    }

    #[test]
    fn test_parse_duration_invalid_number() {
        let result = parse_duration("abch");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid duration number"));
    }

    #[test]
    fn test_parse_duration_negative() {
        let result = parse_duration("-5d");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_parse_duration_zero() {
        let result = parse_duration("0h");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_validate_conversation_number_valid() {
        assert!(validate_conversation_number(1).is_ok());
        assert!(validate_conversation_number(100).is_ok());
        assert!(validate_conversation_number(999999).is_ok());
    }

    #[test]
    fn test_validate_conversation_number_zero() {
        let result = validate_conversation_number(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_validate_conversation_number_negative() {
        let result = validate_conversation_number(-5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_validate_conversation_numbers_valid() {
        assert!(validate_conversation_numbers(&[1, 2, 3]).is_ok());
        assert!(validate_conversation_numbers(&[100]).is_ok());
        assert!(validate_conversation_numbers(&[]).is_ok());
    }

    #[test]
    fn test_validate_conversation_numbers_invalid() {
        let result = validate_conversation_numbers(&[1, 0, 3]);
        assert!(result.is_err());

        let result = validate_conversation_numbers(&[-1, 2, 3]);
        assert!(result.is_err());
    }
}
