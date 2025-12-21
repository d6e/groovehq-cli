mod api;
mod cli;
mod config;
mod error;
mod types;

use anyhow::Context;
use chrono::{Duration, Utc};
use clap::Parser;
use std::io::{self, IsTerminal, Read};

use api::GrooveClient;
use cli::{
    CannedRepliesAction, Commands, ConfigAction, ConversationAction, FolderAction, OutputFormat,
    TagAction, print_completions,
};
use config::Config;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err}");

        if std::env::var("GROOVE_DEBUG").is_ok() {
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
    let cli = cli::Cli::parse();
    let config = Config::load().context("Failed to load configuration")?;

    match &cli.command {
        Commands::Config { action } => handle_config(action, &config, cli.quiet)?,
        Commands::Completions { shell } => {
            print_completions(*shell);
        }
        _ => {
            let token = config::resolve_token(cli.token.as_deref(), &config)?;
            let client = GrooveClient::new(&token, config.api_endpoint.as_deref())?;
            handle_command(&cli.command, &client, &cli.format, &config, cli.quiet).await?;
        }
    }

    Ok(())
}

fn handle_config(action: &ConfigAction, config: &Config, quiet: bool) -> anyhow::Result<()> {
    match action {
        ConfigAction::Show => {
            if let Some(token) = &config.api_token {
                let masked = if token.len() >= 8 {
                    format!("{}...{}", &token[..4], &token[token.len() - 4..])
                } else {
                    "***".to_string()
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
            let limit = limit.or(config.defaults.limit).unwrap_or(25);
            let response = client
                .conversations(
                    Some(limit),
                    after.clone(),
                    status.as_deref(),
                    folder.as_deref(),
                    search.as_deref(),
                )
                .await?;
            cli::format_conversations(&response, format);
        }

        ConversationAction::View { number, full } => {
            validate_conversation_number(*number)?;
            let conv = client.conversation(*number).await?;
            let messages = client.messages(&conv.id, None).await?;
            cli::format_conversation_detail(&conv, &messages, *full);
        }

        ConversationAction::Reply { number, body, canned } => {
            validate_conversation_number(*number)?;
            let body = if let Some(canned_name) = canned {
                // Look up canned reply
                let canned_replies = client.canned_replies().await?;
                let canned_reply = canned_replies
                    .iter()
                    .find(|r| r.name.eq_ignore_ascii_case(canned_name) || r.id == *canned_name)
                    .ok_or_else(|| error::GrooveError::CannedReplyNotFound(canned_name.clone()))?;

                let canned_body = canned_reply.body.clone().unwrap_or_default();

                // Optionally append custom text
                if let Some(extra) = body {
                    format!("{}\n\n{}", canned_body, extra)
                } else {
                    canned_body
                }
            } else {
                get_body(body.clone())?
            };

            let conv = client.conversation(*number).await?;
            client.reply(&conv.id, &body).await?;
            if !quiet {
                println!("Reply sent to conversation #{}", number);
            }
        }

        ConversationAction::Close { numbers } => {
            validate_conversation_numbers(numbers)?;
            for number in numbers {
                let conv = client.conversation(*number).await?;
                client.close(&conv.id).await?;
                if !quiet {
                    println!("Closed conversation #{}", number);
                }
            }
        }

        ConversationAction::Open { numbers } => {
            validate_conversation_numbers(numbers)?;
            for number in numbers {
                let conv = client.conversation(*number).await?;
                client.open(&conv.id).await?;
                if !quiet {
                    println!("Opened conversation #{}", number);
                }
            }
        }

        ConversationAction::Snooze { number, duration } => {
            validate_conversation_number(*number)?;
            let until = parse_duration(duration)?;
            let conv = client.conversation(*number).await?;
            client.snooze(&conv.id, &until).await?;
            if !quiet {
                println!("Snoozed conversation #{} until {}", number, until);
            }
        }

        ConversationAction::Assign { number, agent } => {
            validate_conversation_number(*number)?;
            let conv = client.conversation(*number).await?;

            let agent_id = if agent == "me" {
                let me = client.me().await?;
                me.id
            } else {
                let agents = client.agents().await?;
                agents
                    .iter()
                    .find(|a| a.email == *agent || a.name.as_deref() == Some(agent))
                    .map(|a| a.id.clone())
                    .ok_or_else(|| error::GrooveError::AgentNotFound(agent.clone()))?
            };

            client.assign(&conv.id, &agent_id).await?;
            if !quiet {
                println!("Assigned conversation #{} to {}", number, agent);
            }
        }

        ConversationAction::Unassign { numbers } => {
            validate_conversation_numbers(numbers)?;
            for number in numbers {
                let conv = client.conversation(*number).await?;
                client.unassign(&conv.id).await?;
                if !quiet {
                    println!("Unassigned conversation #{}", number);
                }
            }
        }

        ConversationAction::AddTag { number, tags } => {
            validate_conversation_number(*number)?;
            let conv = client.conversation(*number).await?;
            let all_tags = client.tags().await?;

            let mut tag_ids = Vec::new();
            for tag_name in tags {
                let tag = all_tags
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(tag_name))
                    .ok_or_else(|| error::GrooveError::TagNotFound(tag_name.to_string()))?;
                tag_ids.push(tag.id.clone());
            }

            client.tag(&conv.id, tag_ids).await?;
            if !quiet {
                println!("Added tags to conversation #{}", number);
            }
        }

        ConversationAction::RemoveTag { number, tags } => {
            validate_conversation_number(*number)?;
            let conv = client.conversation(*number).await?;
            let all_tags = client.tags().await?;

            let mut tag_ids = Vec::new();
            for tag_name in tags {
                let tag = all_tags
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(tag_name))
                    .ok_or_else(|| error::GrooveError::TagNotFound(tag_name.to_string()))?;
                tag_ids.push(tag.id.clone());
            }

            client.untag(&conv.id, tag_ids).await?;
            if !quiet {
                println!("Removed tags from conversation #{}", number);
            }
        }

        ConversationAction::Note { number, body } => {
            validate_conversation_number(*number)?;
            let body = get_body(body.clone())?;
            let conv = client.conversation(*number).await?;
            client.add_note(&conv.id, &body).await?;
            if !quiet {
                println!("Note added to conversation #{}", number);
            }
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
    // If it looks like an ISO datetime, return as-is
    if s.contains('T') || s.contains('-') {
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
        assert!(result.unwrap_err().to_string().contains("Invalid duration unit"));
    }

    #[test]
    fn test_parse_duration_invalid_number() {
        let result = parse_duration("abch");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid duration number"));
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