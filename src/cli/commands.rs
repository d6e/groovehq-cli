use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "groove")]
#[command(author, version, about = "GrooveHQ CLI - Manage your inbox from the terminal")]
pub struct Cli {
    /// Output format
    #[arg(long, global = true, default_value = "table")]
    pub format: OutputFormat,

    /// API token (overrides config file and env var)
    #[arg(long, global = true, hide_env_values = true)]
    pub token: Option<String>,

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

        /// Number of results to show
        #[arg(short = 'n', long, default_value = "25")]
        limit: i32,

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

    /// Add tags to a conversation
    #[command(alias = "tag")]
    AddTag {
        /// Conversation number
        number: i64,

        /// Tag names to add
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
