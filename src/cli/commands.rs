use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};

#[derive(Parser)]
#[command(name = "groove")]
#[command(author, version, about = "GrooveHQ CLI - Manage your inbox from the terminal")]
pub struct Cli {
    /// Output format (table, json, compact)
    #[arg(long, global = true)]
    pub format: Option<OutputFormat>,

    /// API token (overrides config file and env var)
    #[arg(long, global = true, hide_env_values = true)]
    pub token: Option<String>,

    /// Suppress success messages (useful for scripting)
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage conversations
    #[command(alias = "conv", alias = "c")]
    Conversation {
        #[command(subcommand)]
        action: ConversationAction,
    },

    /// List and manage folders
    #[command(alias = "f")]
    Folder {
        #[command(subcommand)]
        action: FolderAction,
    },

    /// List and manage tags
    #[command(alias = "t")]
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },

    /// List canned replies
    #[command(alias = "canned")]
    CannedReplies {
        #[command(subcommand)]
        action: CannedRepliesAction,
    },

    /// Show current user info
    Me,

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum ConversationAction {
    /// List conversations
    #[command(alias = "ls", alias = "l")]
    List {
        /// Filter by status (open, closed, snoozed, unread)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by folder name or ID
        #[arg(short, long)]
        folder: Option<String>,

        /// Search by keyword in subject/body
        #[arg(short = 'q', long)]
        search: Option<String>,

        /// Number of results to show (default: 25, or from config)
        #[arg(short = 'n', long)]
        limit: Option<u32>,

        /// Cursor for pagination
        #[arg(long)]
        after: Option<String>,
    },

    /// Show a specific conversation with messages
    #[command(alias = "show", alias = "v")]
    View {
        /// Conversation number
        number: i64,

        /// Show full message bodies (not truncated)
        #[arg(long)]
        full: bool,
    },

    /// Reply to a conversation
    #[command(alias = "r")]
    Reply {
        /// Conversation number
        number: i64,

        /// Reply body (reads from stdin if not provided)
        body: Option<String>,

        /// Use a canned reply by name or ID
        #[arg(short, long)]
        canned: Option<String>,
    },

    /// Close a conversation
    Close {
        /// Conversation number(s)
        numbers: Vec<i64>,
    },

    /// Reopen a conversation
    Open {
        /// Conversation number(s)
        numbers: Vec<i64>,
    },

    /// Snooze a conversation
    Snooze {
        /// Conversation number
        number: i64,

        /// Snooze duration (e.g., "1h", "2d", "1w") or ISO datetime
        duration: String,
    },

    /// Assign a conversation to an agent
    Assign {
        /// Conversation number
        number: i64,

        /// Agent email or "me" for self-assignment
        agent: String,
    },

    /// Unassign a conversation
    Unassign {
        /// Conversation number(s)
        numbers: Vec<i64>,
    },

    /// Add tags to a conversation
    #[command(alias = "tag")]
    AddTag {
        /// Conversation number
        number: i64,

        /// Tag names to add
        tags: Vec<String>,
    },

    /// Remove tags from a conversation
    #[command(alias = "untag")]
    RemoveTag {
        /// Conversation number
        number: i64,

        /// Tag names to remove
        tags: Vec<String>,
    },

    /// Add a private note to a conversation
    Note {
        /// Conversation number
        number: i64,

        /// Note body (reads from stdin if not provided)
        body: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum FolderAction {
    /// List all folders
    #[command(alias = "ls", alias = "l")]
    List,
}

#[derive(Subcommand)]
pub enum TagAction {
    /// List all tags
    #[command(alias = "ls", alias = "l")]
    List,
}

#[derive(Subcommand)]
pub enum CannedRepliesAction {
    /// List all canned replies
    #[command(alias = "ls", alias = "l")]
    List,

    /// Show a specific canned reply
    Show {
        /// Canned reply name or ID
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Set API token
    SetToken {
        /// API token value
        token: String,
    },

    /// Show config file path
    Path,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Compact,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "compact" => Ok(OutputFormat::Compact),
            _ => Err(format!("Invalid format: {}. Use table, json, or compact", s)),
        }
    }
}

pub fn print_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "groove", &mut std::io::stdout());
}
