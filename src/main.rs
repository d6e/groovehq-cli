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
    TagAction,
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
        Commands::Config { action } => handle_config(action, &config)?,
        _ => {
            let token = config::resolve_token(cli.token.as_deref(), &config)?;
            let client = GrooveClient::new(&token, config.api_endpoint.as_deref())?;
            handle_command(&cli.command, &client, &cli.format, &config).await?;
        }
    }

    Ok(())
}

fn handle_config(action: &ConfigAction, config: &Config) -> anyhow::Result<()> {
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
            println!("Token saved successfully");
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
) -> anyhow::Result<()> {
    match command {
        Commands::Me => {
            let agent = client.me().await?;
            cli::format_agent(&agent, format);
        }

        Commands::Conversation { action } => {
            handle_conversation(action, client, format, config).await?;
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

        Commands::Config { .. } => unreachable!(),
    }

    Ok(())
}

async fn handle_conversation(
    action: &ConversationAction,
    client: &GrooveClient,
    format: &OutputFormat,
    config: &Config,
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
            let conv = client.conversation(*number).await?;
            let messages = client.messages(&conv.id, None).await?;
            cli::format_conversation_detail(&conv, &messages, *full);
        }

        ConversationAction::Reply { number, body } => {
            let body = get_body(body.clone())?;
            let conv = client.conversation(*number).await?;
            client.reply(&conv.id, &body).await?;
            println!("Reply sent to conversation #{}", number);
        }

        ConversationAction::Close { numbers } => {
            for number in numbers {
                let conv = client.conversation(*number).await?;
                client.close(&conv.id).await?;
                println!("Closed conversation #{}", number);
            }
        }

        ConversationAction::Open { numbers } => {
            for number in numbers {
                let conv = client.conversation(*number).await?;
                client.open(&conv.id).await?;
                println!("Opened conversation #{}", number);
            }
        }

        ConversationAction::Snooze { number, duration } => {
            let until = parse_duration(duration)?;
            let conv = client.conversation(*number).await?;
            client.snooze(&conv.id, &until).await?;
            println!("Snoozed conversation #{} until {}", number, until);
        }

        ConversationAction::Assign { number, agent } => {
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
            println!("Assigned conversation #{} to {}", number, agent);
        }

        ConversationAction::AddTag { number, tags } => {
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
            println!("Added tags to conversation #{}", number);
        }

        ConversationAction::Note { number, body } => {
            let body = get_body(body.clone())?;
            let conv = client.conversation(*number).await?;
            client.add_note(&conv.id, &body).await?;
            println!("Note added to conversation #{}", number);
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