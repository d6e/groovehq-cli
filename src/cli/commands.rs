use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};

#[derive(Parser)]
#[command(name = "groove")]
#[command(
    author,
    version,
    about = "GrooveHQ CLI - Manage your inbox from the terminal"
)]
#[command(after_help = "EXAMPLES:
    groove conversation list --status open
    groove conversation view 12345
    groove conversation reply 12345 \"Thanks for reaching out!\"
    groove config show")]
pub struct Cli {
    /// Output format (table, json, compact)
    #[arg(long, short = 'o', global = true)]
    pub format: Option<OutputFormat>,

    /// API token (overrides config file and env var)
    #[arg(long, global = true, hide_env_values = true)]
    pub token: Option<String>,

    /// Suppress success messages (useful for scripting)
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Show detailed error information
    #[arg(long, short, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage conversations
    #[command(alias = "conv", alias = "c", after_help = "EXAMPLES:
    groove conversation list --status open --limit 10
    groove conversation view 12345 --full
    groove conversation reply 12345 \"Thank you!\"
    groove conversation close 12345 12346")]
    Conversation {
        #[command(subcommand)]
        action: ConversationAction,
    },

    /// List and manage folders
    #[command(alias = "f", after_help = "EXAMPLES:
    groove folder list")]
    Folder {
        #[command(subcommand)]
        action: FolderAction,
    },

    /// List and manage tags
    #[command(alias = "t", after_help = "EXAMPLES:
    groove tag list")]
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },

    /// List canned replies
    #[command(alias = "canned", after_help = "EXAMPLES:
    groove canned-replies list
    groove canned-replies show \"greeting\"")]
    CannedReplies {
        #[command(subcommand)]
        action: CannedRepliesAction,
    },

    /// Show current user info
    #[command(after_help = "EXAMPLES:
    groove me")]
    Me,

    /// Manage configuration
    #[command(alias = "cfg", after_help = "EXAMPLES:
    groove config show
    groove config set-token abc123
    groove config path")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Generate shell completions
    #[command(after_help = "EXAMPLES:
    groove completions bash > ~/.bash_completion.d/groove
    groove completions zsh > ~/.zfunc/_groove
    groove completions fish > ~/.config/fish/completions/groove.fish")]
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum ConversationAction {
    /// List conversations
    #[command(alias = "ls", alias = "l", after_help = "EXAMPLES:
    groove conversation list
    groove conversation list --status open --folder inbox
    groove conversation list --search \"password reset\" --limit 10")]
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
    #[command(alias = "show", alias = "v", after_help = "EXAMPLES:
    groove conversation view 12345
    groove conversation view 12345 --full")]
    View {
        /// Conversation number
        number: i64,

        /// Show full message bodies (not truncated)
        #[arg(long)]
        full: bool,
    },

    /// Reply to a conversation
    #[command(alias = "r", after_help = "EXAMPLES:
    groove conversation reply 12345 \"Thanks for your message!\"
    groove conversation reply 12345 --canned greeting
    echo \"Reply body\" | groove conversation reply 12345")]
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
    #[command(after_help = "EXAMPLES:
    groove conversation close 12345
    groove conversation close 12345 12346 12347")]
    Close {
        /// Conversation number(s)
        numbers: Vec<i64>,
    },

    /// Reopen a conversation
    #[command(after_help = "EXAMPLES:
    groove conversation open 12345")]
    Open {
        /// Conversation number(s)
        numbers: Vec<i64>,
    },

    /// Snooze a conversation
    #[command(after_help = "EXAMPLES:
    groove conversation snooze 12345 1h
    groove conversation snooze 12345 2d
    groove conversation snooze 12345 2025-01-15T10:00:00")]
    Snooze {
        /// Conversation number
        number: i64,

        /// Snooze duration (e.g., "1h", "2d", "1w") or ISO datetime
        duration: String,
    },

    /// Assign a conversation to an agent
    #[command(after_help = "EXAMPLES:
    groove conversation assign 12345 me
    groove conversation assign 12345 user@example.com")]
    Assign {
        /// Conversation number
        number: i64,

        /// Agent email or "me" for self-assignment
        agent: String,
    },

    /// Unassign a conversation
    #[command(after_help = "EXAMPLES:
    groove conversation unassign 12345")]
    Unassign {
        /// Conversation number(s)
        numbers: Vec<i64>,
    },

    /// Add tags to a conversation
    #[command(alias = "tag", after_help = "EXAMPLES:
    groove conversation add-tag 12345 urgent
    groove conversation add-tag 12345 bug feature")]
    AddTag {
        /// Conversation number
        number: i64,

        /// Tag names to add
        tags: Vec<String>,
    },

    /// Remove tags from a conversation
    #[command(alias = "untag", after_help = "EXAMPLES:
    groove conversation remove-tag 12345 urgent")]
    RemoveTag {
        /// Conversation number
        number: i64,

        /// Tag names to remove
        tags: Vec<String>,
    },

    /// Add a private note to a conversation
    #[command(after_help = "EXAMPLES:
    groove conversation note 12345 \"Internal note about this ticket\"
    echo \"Note body\" | groove conversation note 12345")]
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
    #[command(alias = "ls", alias = "l", after_help = "EXAMPLES:
    groove folder list")]
    List,
}

#[derive(Subcommand)]
pub enum TagAction {
    /// List all tags
    #[command(alias = "ls", alias = "l", after_help = "EXAMPLES:
    groove tag list")]
    List,
}

#[derive(Subcommand)]
pub enum CannedRepliesAction {
    /// List all canned replies
    #[command(alias = "ls", alias = "l", after_help = "EXAMPLES:
    groove canned-replies list")]
    List,

    /// Show a specific canned reply
    #[command(after_help = "EXAMPLES:
    groove canned-replies show greeting
    groove canned-replies show \"thank you\"")]
    Show {
        /// Canned reply name or ID
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Interactive configuration setup
    #[command(after_help = "EXAMPLES:
    groove config init")]
    Init,

    /// Show current configuration
    #[command(after_help = "EXAMPLES:
    groove config show")]
    Show,

    /// Set API token
    #[command(after_help = "EXAMPLES:
    groove config set-token your-api-token-here")]
    SetToken {
        /// API token value
        token: String,
    },

    /// Show config file path
    #[command(after_help = "EXAMPLES:
    groove config path")]
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
            _ => Err(format!(
                "Invalid format: {}. Use table, json, or compact",
                s
            )),
        }
    }
}

pub fn print_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "groove", &mut std::io::stdout());
}
