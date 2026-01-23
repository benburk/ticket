# tk - A Minimal Ticket Tracker

A lightweight, git-backed ticket tracker following Unix philosophy. Tickets are stored as Markdown files with YAML frontmatter, making them human-readable and merge-friendly.

## Install

### Cargo

```bash
cargo install --path .
```

### NixOS / Nix

```bash
# Build
nix build .#tk

# Install to profile
nix profile add /path/to/ticket#tk

# Upgrade after changes
nix profile upgrade tk
```

## Quick Start

```bash
tk init                    # Initialize .tickets/
tk create "My first task"  # Create a ticket
tk list                    # See all tickets
tk ready                   # What can I work on?
```

## Typical Workflow

```bash
# Start a project
tk init
tk create "Set up database schema" --type feat
tk create "Implement user auth" --type feat
tk create "Fix login timeout bug" --type fix --priority 1

# Add dependencies (auth needs db first)
tk dep tk-auth tk-db       # auth depends on db

# Check what's ready to work on
tk ready
# tk-db   [P2] Set up database schema
# tk-fix  [P1] Fix login timeout bug

# Start working
tk start tk-db
tk list
# [>] tk-db   [P2] Set up database schema
# [B] tk-auth [P2] Implement user auth
# [ ] tk-fix  [P1] Fix login timeout bug

# Add notes as you work
tk note tk-db "Added users and sessions tables"
tk note tk-db "Need to add indexes for performance"

# Finish and move on
tk close tk-db
tk ready
# tk-auth [P2] Implement user auth  <- now unblocked!
# tk-fix  [P1] Fix login timeout bug

# Commit your tickets with your code
git add .tickets && git commit -m "Track project tasks"
```

## Commands

| Command               | Description                      |
| --------------------- | -------------------------------- |
| `init`                | Initialize `.tickets/` directory |
| `create <title>`      | Create a new ticket              |
| `list`                | List all tickets                 |
| `show <id>`           | Show ticket details              |
| `edit <id>`           | Edit ticket in `$EDITOR`         |
| `start <id>`          | Mark ticket as in-progress       |
| `close <id>`          | Close a ticket                   |
| `reopen <id>`         | Reopen a closed ticket           |
| `dep <id> <dep-id>`   | Add blocking dependency          |
| `undep <id> <dep-id>` | Remove dependency                |
| `ready`               | List tickets ready to work on    |
| `blocked`             | List blocked tickets             |
| `dep-cycle`           | Detect dependency cycles         |
| `tree <id>`           | Show dependency tree             |
| `note <id> <text>`    | Add timestamped note             |
| `archive <id>`        | Move to archive                  |
| `unarchive <id>`      | Restore from archive             |
| `delete <id>`         | Permanently delete               |
| `prime`               | Output agent instructions        |
| `query`               | Output JSON for piping to `jq`   |

## Concepts

**deps** - Blocking dependencies. A ticket with open deps appears in `blocked`, not `ready`.

**ready** - Open tickets with no unresolved dependencies.

**blocked** - Open tickets waiting on dependencies.

## Ticket Format

Tickets are Markdown files with YAML frontmatter:

```markdown
---
id: tk-a1b2
status: open
type: feat
priority: 2
created: 2024-01-15T10:30:00Z
deps:
  - tk-c3d4
tags:
  - backend
---

# Implement user authentication

Add JWT-based auth with refresh tokens.

[2024-01-15 10:45 alice] Started research on JWT libraries
[2024-01-15 14:30 alice] Going with jsonwebtoken crate
```

## Options

```bash
tk --json list           # JSON output for scripting
tk list --all            # Include archived tickets
tk list --status open    # Filter by status
tk list --tag backend    # Filter by tag
tk list --tag a,b        # Multiple tags (AND logic)
tk ready --tag backend   # Ready tickets with tag
tk blocked --tag urgent  # Blocked tickets with tag
```

## Types

Ticket types align with [conventional commits](https://www.conventionalcommits.org/):

| Type       | Semver | Description                     |
| ---------- | ------ | ------------------------------- |
| `feat`     | MINOR  | New feature or capability       |
| `fix`      | PATCH  | Bug fix                         |
| `chore`    | -      | Maintenance, dependencies       |
| `docs`     | -      | Documentation only              |
| `refactor` | -      | Code change, no behavior change |
| `test`     | -      | Test coverage                   |

This enables: ticket type → commit type → semver → changelog.

## Integration with jq

```bash
# High priority fixes
tk query | jq '.[] | select(.type=="fix" and .priority==1)'

# Count by status
tk query --all | jq 'group_by(.status) | map({status: .[0].status, count: length})'

# Export to CSV
tk query | jq -r '.[] | [.id, .title, .status] | @csv'
```

## Agent Integration

Use `tk prime` to output instructions for AI agents:

```bash
tk prime           # Outputs usage guide + project state
tk prime | head    # First few lines for context window
```

The prime command outputs:

- Key concepts (deps, ready, blocked)
- Common workflow
- Command reference
- Current project state (if initialized)

## Philosophy

- **File per ticket** - Clean git diffs, easy merges
- **Human readable** - Markdown + YAML, edit with any tool
- **Git native** - No sync, no server, just files
- **Unix friendly** - JSON output, pipes to standard tools
- **Minimal** - Does one thing well
