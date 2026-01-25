# tk

A lightweight, git-backed ticket tracker that stores tickets as Markdown files in your repository. No external services, no databases—just files that version with your code.

## Install

```bash
# Cargo
cargo install --path .

# Nix
nix profile install github:cullback/ticket#tk
```

## Quick Start

![Demo](assets/demo.gif)

```bash
tk init                              # Create .tickets/ directory
echo "# Fix login bug" | tk create   # Create a ticket from stdin
tk list                              # See all tickets
tk ready                             # See what's ready to work on
tk close tk-a1b2                     # Close a ticket
```

## Key Features

- **Git-native storage** — Each ticket is a Markdown file in `.tickets/`, making diffs readable and merges easy
- **Dependency tracking** — Model blocking relationships with `tk dep` and see what's actionable with `tk ready`
- **Tags** — Organize tickets with `--tags backend,urgent` and filter with `--tag backend`
- **Unix-friendly** — All commands support `--json` for piping to `jq` and other tools
- **Offline-first** — No server, no sync, no account; tickets live in your repo
- **Prefix matching** — Reference `tk-a1b2c3d4` as just `tk-a1` when unambiguous

## Usage

```
tk [OPTIONS] <COMMAND>

Commands:
  init       Initialize ticket tracking in current directory
  create     Create a new ticket from stdin (expects "# Title" on first line)
  list       List tickets
  show       Show a ticket
  edit       Replace ticket title + body from stdin (expects "# Title" on first line)
  status     Change ticket status
  close      Close a ticket
  reopen     Reopen a ticket
  dep        Add a blocking dependency
  undep      Remove a blocking dependency
  ready      List tickets ready to work on (open, no unresolved deps)
  blocked    List blocked tickets (open, has unresolved deps)
  dep-cycle  Detect dependency cycles
  tree       Show dependency tree (all tickets if no ID given)
  note       Add a timestamped note to a ticket
  query      Query tickets as JSON (pipe to jq)
  help       Print this message or the help of the given subcommand(s)

Options:
      --json     Output in JSON format
  -h, --help     Print help
  -V, --version  Print version
```

Run `tk <command> --help` for command-specific options.

## Ticket Format

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

## Philosophy

See [PHILOSOPHY.md](PHILOSOPHY.md) for the design rationale and recommended git workflow.

## Inspiration

- [beads](https://github.com/steveyegge/beads)
- [beans](https://github.com/hmans/beans)
- [ticket](https://github.com/wedow/ticket)
- [Backlog.md](https://github.com/MrLesk/Backlog.md)
