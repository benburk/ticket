# Why These Design Decisions?

This document explains the rationale behind tk's design choices, drawing from lessons learned across similar projects (beads, beans, ticket-bash).

## Why Markdown + YAML Instead of JSONL?

JSONL (one JSON object per line) seems ideal for machine parsing, but has problems:

- **Context window bloat**: A single JSONL line with a long description dumps thousands of characters into an AI's context just to read one ticket
- **Poor searchability**: grep/ripgrep work, but results are walls of JSON
- **Merge conflicts**: Long lines conflict more often than short ones
- **Human-hostile**: Try editing a 2000-character JSON line in vim

Markdown with YAML frontmatter gives us:

- **AI-friendly**: Agents can search ticket content with standard tools, read individual files without parsing the entire database
- **Human-friendly**: Edit tickets in any text editor, readable in GitHub/GitLab UI
- **Git-friendly**: Changes diff cleanly, line-by-line
- **IDE integration**: Tickets render as documentation

## Why File-Per-Ticket Instead of Single Database?

A single `issues.jsonl` file seems simpler, but:

- **Lock contention**: Multiple processes writing = corruption risk
- **Merge pain**: Two people adding tickets = manual merge of same file
- **Context cost**: Reading one ticket requires parsing the entire file (or maintaining an index)

File-per-ticket provides:

- **Atomic operations**: Write one file, no locks needed
- **Natural git merges**: Different tickets = different files = auto-merge
- **IDE navigation**: Ctrl+click ticket ID in code → jump to file
- **Partial reads**: Load only tickets you need

## Why Hash-Based IDs Instead of Sequential?

Sequential IDs (`#1`, `#2`, `#3`) seem intuitive, but:

- **Merge conflicts**: Two branches both create `#42` → conflict
- **Coordination required**: Need central authority to assign numbers
- **Predictable**: Competitors can enumerate your ticket count

Hash-based IDs (`tk-a1b2`) provide:

- **Conflict-free creation**: UUID → SHA256 → truncate = practically unique
- **Parallel-safe**: Multiple agents create tickets simultaneously on different branches
- **Automatic scaling**: Start with 4 hex chars, extend if collisions occur
- **Opaque**: No information leakage about project size

## Why No SQLite Cache?

Beads uses SQLite as a query cache over JSONL. We skip it because:

- **Complexity**: Background daemon, sync logic, cache invalidation
- **Failure modes**: Stale cache, corrupted index, daemon crashes
- **Context cost**: Agents must understand two systems (cache + source of truth)

Direct file access is sufficient when:

- **Scale is modest**: <1000 tickets? Reading files is fast enough
- **Queries are simple**: "What's ready?" doesn't need SQL
- **Agents use grep**: AI agents are great at searching text files

Trade-off: We sacrifice millisecond queries for conceptual simplicity.

## Why No Background Daemon?

Beads runs a daemon for batching git operations and holding connections. We skip it because:

- **Hidden state**: Daemon holds uncommitted changes invisibly
- **Process management**: Starting, stopping, crash recovery
- **Port conflicts**: Multiple workspaces compete for sockets

Without a daemon:

- **Explicit is better**: `git add .tickets && git commit` is visible and intentional
- **No surprises**: Filesystem state = truth, always
- **Debuggable**: Something wrong? Just look at the files

## Why Blocking Deps vs Symmetric Links?

Most trackers have one relationship type. We have two:

**Deps (blocking)**: "This ticket cannot start until that one closes"

- Directional: A depends on B (not vice versa)
- Affects workflow: blocked tickets filtered from `ready`
- Common: "implement auth" depends on "set up database"

**Links (symmetric)**: "These tickets are related"

- Bidirectional: A links to B means B links to A
- Informational only: doesn't block anything
- Common: "fix login bug" relates to "fix logout bug"

This distinction matters because:

- **Ready/blocked commands work correctly**: Only true blockers affect workflow
- **Agents can reason about dependencies**: Clear semantics for planning
- **Humans see relationships**: Without cluttering the dependency graph

## Why Ready and Blocked Commands?

Traditional trackers show status (open/closed). We show workflow state:

- **ready**: Open tickets with no unresolved deps → what can I work on?
- **blocked**: Open tickets waiting on deps → what's stuck?

This is the killer feature for agents:

- Agent asks "what's ready?" not "show me all open tickets and let me figure out which are blocked"
- Workflow-oriented queries reduce context usage
- Prioritization becomes obvious (ready + high priority = do this)

## Why Archive Instead of Delete?

Delete is permanent. Archive is reversible:

- **Safety**: Accidentally archived? Restore it
- **History**: Completed work remains searchable
- **Compliance**: Some organizations require ticket retention
- **Context**: Agents can search archived tickets for project memory

Archive moves files to `.tickets/archive/`. Delete removes them forever.

## Why Rust Instead of Bash?

The reference `ticket` project is ~900 lines of bash. We chose Rust because:

- **Type safety**: Catch errors at compile time, not runtime
- **Error handling**: `Result<T, E>` forces handling failures
- **Performance**: Not that bash is slow, but Rust is faster
- **Tooling**: cargo, clippy, rustfmt, rust-analyzer
- **Portability**: Single binary, no bash version differences

Trade-off: Higher barrier to contribution than a shell script.

## Why Rust Instead of Go?

The `beans` project is Go. We chose Rust because:

- **No runtime**: Single static binary, no GC pauses
- **Stronger types**: Enums, pattern matching, Option/Result
- **Memory safety**: No nil pointer panics
- **Personal preference**: The author likes Rust

Honest trade-off: Go would have been fine. This is mostly taste.

## Why Clap for CLI?

Clap (derive macros) gives us:

- **Type-safe arguments**: Parsed directly into structs
- **Auto-generated help**: `--help` works correctly, always
- **Completions**: Shell completions for free
- **Subcommands**: Natural `tk create`, `tk list` structure

## Why YAML Frontmatter Instead of TOML?

Both work. YAML wins because:

- **Ecosystem**: Jekyll, Hugo, Obsidian all use YAML frontmatter
- **Familiarity**: More developers know YAML than TOML
- **Tooling**: `serde_yaml` just works

TOML would have been slightly cleaner syntax but less familiar.

## Why Priority 1-3 Instead of Labels?

Labels are flexible but chaotic:

- Is "urgent" higher than "critical"?
- What's the difference between "important" and "high-priority"?
- Labels accumulate cruft over time

Numeric priority is simple:

- **P1**: Do this now
- **P2**: Do this soon (default)
- **P3**: Do this eventually

Agents can sort by priority without understanding label semantics.

## Why Types (task/bug/feature/epic/chore)?

Minimal taxonomy that covers most needs:

- **task**: Generic work item
- **bug**: Something broken
- **feature**: New capability
- **epic**: Large effort containing subtasks
- **chore**: Maintenance, refactoring, dependencies

Not included: story, spike, tech-debt, improvement, enhancement. These map to the above.

## Why JSON Output Flag?

Every command supports `--json` because:

- **Composability**: Pipe to `jq` for arbitrary queries
- **Scripting**: Build automation on top of tk
- **Agent-friendly**: Structured output for programmatic consumption

Example: `tk query | jq '.[] | select(.type=="bug" and .priority==1)'`

## Why `tk` Instead of `ticket`?

- **Speed**: Two characters vs six
- **Muscle memory**: `tk create "Fix bug"` flows
- **Namespace**: Unlikely to conflict with existing commands
- **Prefix convention**: Similar to `bd` (beads), `gh` (GitHub CLI)

## Why Store in `.tickets/` Not `.tk/`?

- **Discoverable**: Seeing `.tickets/` immediately explains what it contains
- **Explicit**: Not an abbreviation that needs lookup
- **Standard**: Follows `.github/`, `.vscode/` conventions

The cost of a few extra characters in the path is worth the clarity.

## Why No Web UI?

Out of scope. This is a CLI tool that:

- **Does one thing well**: Manage tickets from terminal
- **Integrates with git**: Which has its own web UIs (GitHub, GitLab)
- **Stays simple**: A web UI would 10x the codebase

If you want a web view, use GitHub's file browser or build a viewer separately.

## Why Not Use GitHub Issues?

GitHub Issues are good, but:

- **External dependency**: Requires network, GitHub account
- **No branching**: Issues don't follow your branch
- **No local-first**: Can't work offline
- **Vendor lock-in**: Migration is painful

Git-native tickets:

- **Branch with code**: Checkout experiment branch, see experiment's tickets
- **Merge with code**: Close tickets when PR merges
- **Work offline**: Full functionality without internet
- **Portable**: Switch hosts freely, tickets come along
