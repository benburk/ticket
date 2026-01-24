# tk

A lightweight, git-backed ticket tracker designed for simplicity.

Tickets are stored as Markdown files with YAML frontmatter in `.tickets/`.
Each ticket is a separate file, making git diffs readable and merges easy.

Searches parent directories for `.tickets/` (override with `TICKETS_DIR` env var).

## Install

```bash
# Cargo
cargo install --path .

# Nix
nix profile install github:cullback/ticket#tk
```

## Usage

```
tk [OPTIONS] <COMMAND>

Commands:
  init       Initialize ticket tracking in current directory
  create     Create a new ticket from stdin (expects "# Title" on first line)
  list       List tickets
  edit       Replace ticket title + body from stdin (expects "# Title" on first line)
  status     Change ticket status
  close      Close a ticket
  reopen     Reopen a ticket
  dep        Add a blocking dependency
  undep      Remove a blocking dependency
  ready      List tickets ready to work on (open, no unresolved deps)
  blocked    List blocked tickets (open, has unresolved deps)
  dep-cycle  Detect dependency cycles
  tree       Show dependency tree for a ticket
  note       Add a timestamped note to a ticket
  query      Query tickets as JSON (pipe to jq)

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
