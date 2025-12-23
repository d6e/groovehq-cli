# GrooveHQ CLI

A command-line interface for managing your [GrooveHQ](https://www.groovehq.com/) inbox.

## Installation

### From Source

```bash
cargo install --path .
```

### Building

```bash
cargo build --release
```

The binary will be located at `target/release/groove`.

## Quick Start

1. Get your API token from GrooveHQ settings
2. Set your token:
   ```bash
   groove config set-token YOUR_API_TOKEN
   ```
   Or use an environment variable:
   ```bash
   export GROOVEHQ_API_TOKEN=YOUR_API_TOKEN
   ```
3. List your conversations:
   ```bash
   groove conv list
   ```

## Commands

### Conversations

```bash
# List conversations
groove conv list
groove conv list --status open
groove conv list --folder inbox --limit 50
groove conv list -q "search term"

# View a conversation with messages
groove conv view 123
groove conv view 123 --full

# Reply to a conversation
groove conv reply 123 "Your message here"
echo "Message from stdin" | groove conv reply 123

# Reply using a canned reply
groove conv reply 123 --canned "Thanks Template"
groove conv reply 123 --canned "Thanks Template" "Additional text"

# Close/reopen conversations
groove conv close 123
groove conv close 123 124 125
groove conv open 123

# Snooze a conversation
groove conv snooze 123 1h    # 1 hour
groove conv snooze 123 2d    # 2 days
groove conv snooze 123 1w    # 1 week

# Assign/unassign
groove conv assign 123 agent@example.com
groove conv assign 123 me
groove conv unassign 123

# Manage tags
groove conv add-tag 123 urgent vip
groove conv remove-tag 123 urgent

# Add a private note
groove conv note 123 "Internal note here"
```

### Folders

```bash
groove folder list
```

### Tags

```bash
groove tag list
```

### Canned Replies

```bash
groove canned-replies list
groove canned-replies show "Template Name"
```

### User Info

```bash
groove me
```

### Configuration

```bash
groove config show
groove config set-token YOUR_TOKEN
groove config path
```

### Shell Completions

```bash
# Bash
groove completions bash > /etc/bash_completion.d/groove

# Zsh
groove completions zsh > ~/.zfunc/_groove

# Fish
groove completions fish > ~/.config/fish/completions/groove.fish

# PowerShell
groove completions powershell > groove.ps1
```

## Options

### Global Options

| Option | Description |
|--------|-------------|
| `--format <FORMAT>` | Output format: `table` (default), `json`, `compact` |
| `--token <TOKEN>` | Override API token |
| `--quiet` | Suppress success messages (useful for scripting) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

### Output Formats

- **table**: Formatted tables with colors (default)
- **json**: Pretty-printed JSON for parsing
- **compact**: One-liner per item for scripting

```bash
# Get conversation data as JSON
groove conv list --format json

# Compact output for scripting
groove conv list --format compact
```

## Configuration

The CLI looks for configuration in these locations (in order of priority):

1. `--token` command line flag
2. `GROOVEHQ_API_TOKEN` environment variable
3. Config file

### Config File Location

- **macOS**: `~/Library/Application Support/groove-cli/config.toml`
- **Linux**: `~/.config/groove-cli/config.toml`
- **Windows**: `%APPDATA%\groove-cli\config\config.toml`

### Config File Format

```toml
api_token = "your-api-token"
api_endpoint = "https://api.groovehq.com/v2/graphql"  # optional

[defaults]
format = "table"
limit = 25
folder = "inbox"
```

## Examples

### Workflow: Process New Conversations

```bash
# List unread conversations
groove conv list --status unread

# View and respond to a conversation
groove conv view 123
groove conv reply 123 "Thank you for reaching out!"
groove conv close 123
```

### Workflow: Bulk Operations

```bash
# Close multiple conversations
groove conv close 100 101 102 103

# Add tags to a conversation
groove conv add-tag 123 priority customer-feedback
```

### Scripting with Quiet Mode

```bash
# Script that closes all conversations matching a pattern
groove --quiet conv close 123
echo $?  # Check exit code for success/failure
```

### JSON Processing with jq

```bash
# Get conversation numbers
groove conv list --format json | jq '.nodes[].number'

# Get customer emails
groove conv list --format json | jq '.nodes[].contact.email'
```

## Command Aliases

For faster typing, these aliases are available:

| Full Command | Aliases |
|--------------|---------|
| `conversation` | `conv`, `c` |
| `folder` | `f` |
| `tag` | `t` |
| `canned-replies` | `canned` |
| `list` | `ls`, `l` |
| `view` | `show`, `v` |
| `reply` | `r` |
| `add-tag` | `tag` |
| `remove-tag` | `untag` |

Examples:
```bash
groove c ls           # Same as: groove conversation list
groove c v 123        # Same as: groove conversation view 123
groove c r 123 "Hi"   # Same as: groove conversation reply 123 "Hi"
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GROOVEHQ_API_TOKEN` | API token for authentication |
| `GROOVE_DEBUG` | Set to any value to show full error traces |

## Development

### Running Tests

```bash
cargo test
```

### Building for Release

```bash
cargo build --release
```

## License

MIT
