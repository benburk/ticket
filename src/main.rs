mod id;
mod storage;
mod types;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::process::Command;
use storage::Storage;
use types::{Note, Status, Ticket, TicketType};

#[derive(Parser)]
#[command(name = "tk")]
#[command(about = "A minimal, Unix-philosophy ticket tracker")]
#[command(
    long_about = "A lightweight, git-backed ticket tracker designed for simplicity.

Tickets are stored as Markdown files with YAML frontmatter in .tickets/.
Each ticket is a separate file, making git diffs readable and merges easy.

Key concepts:
  - deps: blocking dependencies (must close dep before this is ready)
  - ready: open tickets with no unresolved deps
  - blocked: open tickets waiting on deps

Workflow:
  1. tk init                        # Initialize .tickets/
  2. echo \"# Task\" | tk create      # Create a ticket
  3. tk ready                       # See what's ready
  4. tk start <id>                  # Mark in-progress
  5. tk close <id>                  # Mark done
  6. git add .tickets && git commit"
)]
#[command(version)]
struct Cli {
    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize ticket tracking in current directory
    Init,

    /// Create a new ticket from stdin (expects "# Title" on first line)
    Create {
        /// Priority (0=critical, 4=backlog)
        #[arg(short, long, default_value = "2")]
        priority: u8,
        /// Type: feat, fix, chore, docs, refactor, test
        #[arg(short = 't', long, default_value = "feat")]
        r#type: String,
        /// Create as child of parent ticket
        #[arg(long)]
        parent: Option<String>,
        /// Initial tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },

    /// List tickets
    #[command(alias = "ls")]
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
        /// Filter by tag (comma-separated for multiple, AND logic)
        #[arg(short = 't', long)]
        tag: Option<String>,
        /// Show archived tickets too
        #[arg(short, long)]
        all: bool,
    },

    /// Show ticket details
    Show {
        /// Ticket ID (prefix match)
        id: String,
    },

    /// Replace ticket title + body from stdin (expects "# Title" on first line)
    Edit {
        /// Ticket ID (prefix match)
        id: String,
    },

    /// Change ticket status
    Status {
        /// Ticket ID (prefix match)
        id: String,
        /// New status: open, in-progress, closed
        status: String,
    },

    /// Start working on a ticket (set in-progress)
    Start {
        /// Ticket ID (prefix match)
        id: String,
    },

    /// Close a ticket
    Close {
        /// Ticket ID (prefix match)
        id: String,
    },

    /// Reopen a ticket
    Reopen {
        /// Ticket ID (prefix match)
        id: String,
    },

    /// Add a blocking dependency
    Dep {
        /// Ticket that is blocked
        id: String,
        /// Ticket that blocks (dependency)
        dep_id: String,
    },

    /// Remove a blocking dependency
    Undep {
        /// Ticket to remove dep from
        id: String,
        /// Dependency to remove
        dep_id: String,
    },

    /// List tickets ready to work on (open, no unresolved deps)
    Ready {
        /// Filter by tag (comma-separated for multiple, AND logic)
        #[arg(short = 't', long)]
        tag: Option<String>,
    },

    /// List blocked tickets (open, has unresolved deps)
    Blocked {
        /// Filter by tag (comma-separated for multiple, AND logic)
        #[arg(short = 't', long)]
        tag: Option<String>,
    },

    /// Detect dependency cycles
    #[command(name = "dep-cycle")]
    DepCycle,

    /// Show dependency tree for a ticket
    Tree {
        /// Ticket ID (prefix match)
        id: String,
        /// Show full tree (include closed)
        #[arg(short, long)]
        full: bool,
    },

    /// Add a timestamped note to a ticket
    Note {
        /// Ticket ID (prefix match)
        id: String,
        /// Note content (opens $EDITOR if omitted)
        content: Option<String>,
    },

    /// Archive a ticket (move to .tickets/archive/)
    Archive {
        /// Ticket ID (prefix match)
        id: String,
    },

    /// Unarchive a ticket
    Unarchive {
        /// Ticket ID (prefix match)
        id: String,
    },

    /// Delete a ticket permanently
    Delete {
        /// Ticket ID (prefix match)
        id: String,
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Query tickets as JSON (pipe to jq)
    Query {
        /// Optional jq-style filter (requires jq)
        filter: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let storage = Storage::new();

    match cli.command {
        Commands::Init => cmd_init(&storage, cli.json),
        Commands::Create {
            priority,
            r#type,
            parent,
            tags,
        } => cmd_create(&storage, priority, &r#type, parent, tags, cli.json),
        Commands::List { status, tag, all } => cmd_list(&storage, status, tag, all, cli.json),
        Commands::Show { id } => cmd_show(&storage, &id, cli.json),
        Commands::Edit { id } => cmd_edit(&storage, &id),
        Commands::Status { id, status } => cmd_status(&storage, &id, &status, cli.json),
        Commands::Start { id } => cmd_status(&storage, &id, "in-progress", cli.json),
        Commands::Close { id } => cmd_close(&storage, &id, cli.json),
        Commands::Reopen { id } => cmd_status(&storage, &id, "open", cli.json),
        Commands::Dep { id, dep_id } => cmd_dep(&storage, &id, &dep_id, cli.json),
        Commands::Undep { id, dep_id } => cmd_undep(&storage, &id, &dep_id, cli.json),
        Commands::Ready { tag } => cmd_ready(&storage, tag, cli.json),
        Commands::Blocked { tag } => cmd_blocked(&storage, tag, cli.json),
        Commands::DepCycle => cmd_dep_cycle(&storage, cli.json),
        Commands::Tree { id, full } => cmd_tree(&storage, &id, full, cli.json),
        Commands::Note { id, content } => cmd_note(&storage, &id, content, cli.json),
        Commands::Archive { id } => cmd_archive(&storage, &id, cli.json),
        Commands::Unarchive { id } => cmd_unarchive(&storage, &id, cli.json),
        Commands::Delete { id, force } => cmd_delete(&storage, &id, force, cli.json),
        Commands::Query { filter } => cmd_query(&storage, filter),
    }
}

fn ensure_init(storage: &Storage) -> Result<()> {
    if !storage.is_initialized() {
        storage.init()?;
        eprintln!("Initialized .tickets/");
    }
    Ok(())
}

fn cmd_init(storage: &Storage, json: bool) -> Result<()> {
    if storage.is_initialized() {
        if json {
            println!(r#"{{"status":"already_initialized"}}"#);
        } else {
            println!("Already initialized.");
        }
        return Ok(());
    }

    storage.init()?;

    if json {
        println!(r#"{{"status":"initialized"}}"#);
    } else {
        println!("Initialized .tickets/");
    }
    Ok(())
}

fn cmd_create(
    storage: &Storage,
    priority: u8,
    type_str: &str,
    parent: Option<String>,
    tags: Option<String>,
    json: bool,
) -> Result<()> {
    use std::io::Read;

    ensure_init(storage)?;

    // Read from stdin
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let input = buf.trim();

    if input.is_empty() {
        anyhow::bail!("No input provided. Expected: # Title\\n[body]");
    }

    // Extract title from first # heading
    let (title, body) = Storage::extract_title(input);
    if title == "Untitled" && !input.starts_with("# ") {
        anyhow::bail!("No title found. First line must be: # Your Title");
    }

    let existing = storage.all_ids()?;

    let (id, parent_id) = if let Some(ref parent_prefix) = parent {
        let parent_ticket = storage
            .find_by_prefix(parent_prefix)?
            .context(format!("Parent '{}' not found", parent_prefix))?;
        let child_id = id::generate_child(parent_ticket.id(), &existing);
        (child_id, Some(parent_ticket.id().to_string()))
    } else {
        (id::generate(&existing), None)
    };

    let ticket_type: TicketType = type_str.parse()?;
    let tags: Vec<String> = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let mut ticket = Ticket::new(id.clone(), title.clone());
    ticket.meta.priority = priority;
    ticket.meta.ticket_type = ticket_type;
    ticket.meta.parent = parent_id;
    ticket.meta.tags = tags;
    ticket.body = body.to_string();

    storage.save(&ticket)?;

    if json {
        println!(r#"{{"id":"{}","title":"{}"}}"#, id, title);
    } else {
        println!("Created {} - {}", id, title);
    }
    Ok(())
}

fn cmd_list(
    storage: &Storage,
    status: Option<String>,
    tag: Option<String>,
    all: bool,
    json: bool,
) -> Result<()> {
    ensure_init(storage)?;

    let tickets = if all {
        storage.load_all_with_archived()?
    } else {
        storage.load_all()?
    };

    let status_filter: Option<Status> = status.map(|s| s.parse()).transpose()?;
    let tags_filter: Vec<String> = tag
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let mut filtered: Vec<_> = tickets
        .iter()
        .filter(|t| status_filter.map_or(true, |s| t.meta.status == s))
        .filter(|t| {
            tags_filter.is_empty() || tags_filter.iter().all(|tag| t.meta.tags.contains(tag))
        })
        .collect();

    filtered.sort_by(|a, b| {
        a.meta
            .priority
            .cmp(&b.meta.priority)
            .then_with(|| a.meta.created.cmp(&b.meta.created))
    });

    if json {
        let items: Vec<_> = filtered
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id(),
                    "title": t.title,
                    "status": t.meta.status.to_string(),
                    "priority": t.meta.priority,
                    "type": t.meta.ticket_type.to_string(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&items)?);
    } else if filtered.is_empty() {
        println!("No tickets found.");
    } else {
        for t in filtered {
            let marker = match t.meta.status {
                Status::Open => " ",
                Status::InProgress => "*",
                Status::Closed => "x",
                Status::Archived => "a",
            };
            println!("[{}] {} [P{}] {}", marker, t.id(), t.meta.priority, t.title);
        }
    }
    Ok(())
}

fn cmd_show(storage: &Storage, id: &str, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    if json {
        let obj = serde_json::json!({
            "id": ticket.id(),
            "title": ticket.title,
            "status": ticket.meta.status.to_string(),
            "priority": ticket.meta.priority,
            "type": ticket.meta.ticket_type.to_string(),
            "deps": ticket.meta.deps,
            "tags": ticket.meta.tags,
            "created": ticket.meta.created,
            "body": ticket.body,
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        println!("ID:       {}", ticket.id());
        println!("Title:    {}", ticket.title);
        println!("Status:   {}", ticket.meta.status);
        println!("Priority: P{}", ticket.meta.priority);
        println!("Type:     {}", ticket.meta.ticket_type);
        if let Some(ref parent) = ticket.meta.parent {
            println!("Parent:   {}", parent);
        }
        println!("Created:  {}", ticket.meta.created.format("%Y-%m-%d %H:%M"));
        if let Some(updated) = ticket.meta.updated {
            println!("Updated:  {}", updated.format("%Y-%m-%d %H:%M"));
        }
        if !ticket.meta.deps.is_empty() {
            println!("Deps:     {}", ticket.meta.deps.join(", "));
        }
        if !ticket.meta.tags.is_empty() {
            println!("Tags:     {}", ticket.meta.tags.join(", "));
        }
        if !ticket.body.is_empty() {
            println!("\n{}", ticket.body);
        }
    }
    Ok(())
}

fn cmd_edit(storage: &Storage, id: &str) -> Result<()> {
    use std::io::Read;

    ensure_init(storage)?;

    let mut ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    // Read title + body from stdin
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let input = buf.trim();

    if input.is_empty() {
        anyhow::bail!("No input provided. Expected: # Title\\n[body]");
    }

    let (title, body) = Storage::extract_title(input);
    if title == "Untitled" && !input.starts_with("# ") {
        anyhow::bail!("No title found. First line must be: # Your Title");
    }

    ticket.title = title;
    ticket.body = body.to_string();
    storage.save(&ticket)?;

    println!("Updated {}", ticket.id());
    Ok(())
}

fn cmd_status(storage: &Storage, id: &str, status_str: &str, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let mut ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    let new_status: Status = status_str.parse()?;
    ticket.meta.status = new_status;
    ticket.touch();

    storage.save(&ticket)?;

    if json {
        println!(r#"{{"id":"{}","status":"{}"}}"#, ticket.id(), new_status);
    } else {
        println!("{} -> {}", ticket.id(), new_status);
    }
    Ok(())
}

fn cmd_close(storage: &Storage, id: &str, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let mut ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    ticket.meta.status = Status::Closed;
    ticket.meta.closed = Some(Utc::now());
    ticket.touch();

    storage.save(&ticket)?;

    if json {
        println!(r#"{{"id":"{}","status":"closed"}}"#, ticket.id());
    } else {
        println!("Closed {}", ticket.id());
    }
    Ok(())
}

fn cmd_dep(storage: &Storage, id: &str, dep_id: &str, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let mut ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    let dep = storage
        .find_by_prefix(dep_id)?
        .context(format!("Dependency '{}' not found", dep_id))?;

    if ticket.meta.deps.contains(&dep.id().to_string()) {
        anyhow::bail!("Dependency already exists");
    }

    ticket.meta.deps.push(dep.id().to_string());
    ticket.touch();
    storage.save(&ticket)?;

    if json {
        println!(r#"{{"id":"{}","dep":"{}"}}"#, ticket.id(), dep.id());
    } else {
        println!("{} now depends on {}", ticket.id(), dep.id());
    }
    Ok(())
}

fn cmd_undep(storage: &Storage, id: &str, dep_id: &str, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let mut ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    let dep = storage
        .find_by_prefix(dep_id)?
        .context(format!("Dependency '{}' not found", dep_id))?;

    let orig_len = ticket.meta.deps.len();
    ticket.meta.deps.retain(|d| d != dep.id());

    if ticket.meta.deps.len() == orig_len {
        anyhow::bail!("Dependency not found");
    }

    ticket.touch();
    storage.save(&ticket)?;

    if json {
        println!(r#"{{"removed":true}}"#);
    } else {
        println!("Removed dependency {} -> {}", ticket.id(), dep.id());
    }
    Ok(())
}

fn cmd_ready(storage: &Storage, tag: Option<String>, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let tickets = storage.load_all()?;
    let tags_filter: Vec<String> = tag
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let mut ready: Vec<_> = tickets
        .iter()
        .filter(|t| t.is_open() && !t.is_blocked_by(&tickets))
        .filter(|t| {
            tags_filter.is_empty() || tags_filter.iter().all(|tag| t.meta.tags.contains(tag))
        })
        .collect();

    ready.sort_by_key(|t| t.meta.priority);

    if json {
        let items: Vec<_> = ready
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id(),
                    "title": t.title,
                    "priority": t.meta.priority,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&items)?);
    } else if ready.is_empty() {
        println!("No ready tickets.");
    } else {
        for t in ready {
            println!("{} [P{}] {}", t.id(), t.meta.priority, t.title);
        }
    }
    Ok(())
}

fn cmd_blocked(storage: &Storage, tag: Option<String>, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let tickets = storage.load_all()?;
    let tags_filter: Vec<String> = tag
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let mut blocked: Vec<_> = tickets
        .iter()
        .filter(|t| t.is_open() && t.is_blocked_by(&tickets))
        .filter(|t| {
            tags_filter.is_empty() || tags_filter.iter().all(|tag| t.meta.tags.contains(tag))
        })
        .collect();

    blocked.sort_by_key(|t| t.meta.priority);

    if json {
        let items: Vec<_> = blocked
            .iter()
            .map(|t| {
                let blocking: Vec<_> = t
                    .meta
                    .deps
                    .iter()
                    .filter(|d| {
                        tickets
                            .iter()
                            .find(|x| x.id() == *d)
                            .map_or(false, |x| x.is_open())
                    })
                    .collect();
                serde_json::json!({
                    "id": t.id(),
                    "title": t.title,
                    "blocked_by": blocking,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&items)?);
    } else if blocked.is_empty() {
        println!("No blocked tickets.");
    } else {
        for t in blocked {
            let blocking: Vec<_> = t
                .meta
                .deps
                .iter()
                .filter(|d| {
                    tickets
                        .iter()
                        .find(|x| x.id() == *d)
                        .map_or(false, |x| x.is_open())
                })
                .cloned()
                .collect();
            println!(
                "{} [P{}] {} (blocked by: {})",
                t.id(),
                t.meta.priority,
                t.title,
                blocking.join(", ")
            );
        }
    }
    Ok(())
}

fn cmd_dep_cycle(storage: &Storage, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let tickets = storage.load_all()?;
    let cycles = find_cycles(&tickets);

    if json {
        println!("{}", serde_json::to_string(&cycles)?);
    } else if cycles.is_empty() {
        println!("No dependency cycles found.");
    } else {
        println!("Dependency cycles detected:");
        for cycle in &cycles {
            println!("  {} -> {}", cycle.join(" -> "), cycle[0]);
        }
    }

    if !cycles.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

/// Find all dependency cycles using DFS
fn find_cycles(tickets: &[Ticket]) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    let ticket_map: HashMap<&str, &Ticket> = tickets.iter().map(|t| (t.id(), t)).collect();

    for ticket in tickets {
        if !visited.contains(ticket.id()) {
            dfs_cycles(
                ticket.id(),
                &ticket_map,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    cycles
}

fn dfs_cycles(
    id: &str,
    tickets: &HashMap<&str, &Ticket>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(id.to_string());
    rec_stack.insert(id.to_string());
    path.push(id.to_string());

    if let Some(ticket) = tickets.get(id) {
        for dep in &ticket.meta.deps {
            if !visited.contains(dep) {
                dfs_cycles(dep, tickets, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(dep) {
                // Found cycle - extract it
                if let Some(start) = path.iter().position(|x| x == dep) {
                    let cycle: Vec<String> = path[start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }
    }

    path.pop();
    rec_stack.remove(id);
}

fn cmd_tree(storage: &Storage, id: &str, full: bool, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let tickets = storage.load_all_with_archived()?;
    let ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    if json {
        let tree = build_tree_json(&ticket, &tickets, full);
        println!("{}", serde_json::to_string_pretty(&tree)?);
    } else {
        println!("{} - {}", ticket.id(), ticket.title);
        print_dep_tree(&ticket, &tickets, "", full);
    }
    Ok(())
}

fn print_dep_tree(ticket: &Ticket, all: &[Ticket], prefix: &str, full: bool) {
    let deps: Vec<_> = ticket
        .meta
        .deps
        .iter()
        .filter_map(|d| all.iter().find(|t| t.id() == d))
        .filter(|t| full || t.is_open())
        .collect();

    for (i, dep) in deps.iter().enumerate() {
        let is_last = i == deps.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let marker = if dep.is_open() { " " } else { "x" };

        println!(
            "{}{}[{}] {} - {}",
            prefix,
            connector,
            marker,
            dep.id(),
            dep.title
        );

        let new_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });
        print_dep_tree(dep, all, &new_prefix, full);
    }
}

fn build_tree_json(ticket: &Ticket, all: &[Ticket], full: bool) -> serde_json::Value {
    let deps: Vec<_> = ticket
        .meta
        .deps
        .iter()
        .filter_map(|d| all.iter().find(|t| t.id() == d))
        .filter(|t| full || t.is_open())
        .map(|t| build_tree_json(t, all, full))
        .collect();

    serde_json::json!({
        "id": ticket.id(),
        "title": ticket.title,
        "status": ticket.meta.status.to_string(),
        "deps": deps,
    })
}

fn cmd_note(storage: &Storage, id: &str, content: Option<String>, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let mut ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    let content = if let Some(c) = content {
        c
    } else {
        // Open editor for note
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let temp = std::env::temp_dir().join(format!("tk-note-{}.md", ticket.id()));
        std::fs::write(&temp, "")?;

        let status = Command::new(&editor).arg(&temp).status()?;
        if !status.success() {
            anyhow::bail!("Editor exited with error");
        }

        let content = std::fs::read_to_string(&temp)?.trim().to_string();
        std::fs::remove_file(&temp)?;

        if content.is_empty() {
            anyhow::bail!("Empty note, aborting");
        }
        content
    };

    let note = Note::new(content);
    let formatted = note.format();

    // Append note to body
    if !ticket.body.is_empty() && !ticket.body.ends_with('\n') {
        ticket.body.push('\n');
    }
    if !ticket.body.is_empty() {
        ticket.body.push('\n');
    }
    ticket.body.push_str(&formatted);
    ticket.touch();

    storage.save(&ticket)?;

    if json {
        println!(r#"{{"added":"{}"}}"#, ticket.id());
    } else {
        println!("Added note to {}", ticket.id());
    }
    Ok(())
}

fn cmd_archive(storage: &Storage, id: &str, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let mut ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    ticket.meta.status = Status::Archived;
    ticket.touch();
    storage.save(&ticket)?;
    storage.archive(ticket.id())?;

    if json {
        println!(r#"{{"archived":"{}"}}"#, ticket.id());
    } else {
        println!("Archived {}", ticket.id());
    }
    Ok(())
}

fn cmd_unarchive(storage: &Storage, id: &str, json: bool) -> Result<()> {
    ensure_init(storage)?;

    storage.unarchive(id)?;

    // Reload and set status to open
    let mut ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    ticket.meta.status = Status::Open;
    ticket.touch();
    storage.save(&ticket)?;

    if json {
        println!(r#"{{"unarchived":"{}"}}"#, ticket.id());
    } else {
        println!("Unarchived {}", ticket.id());
    }
    Ok(())
}

fn cmd_delete(storage: &Storage, id: &str, force: bool, json: bool) -> Result<()> {
    ensure_init(storage)?;

    let ticket = storage
        .find_by_prefix(id)?
        .context(format!("Ticket '{}' not found", id))?;

    if !force {
        eprintln!(
            "Delete {} - {}? Use --force to confirm.",
            ticket.id(),
            ticket.title
        );
        std::process::exit(1);
    }

    storage.delete(ticket.id())?;

    if json {
        println!(r#"{{"deleted":"{}"}}"#, ticket.id());
    } else {
        println!("Deleted {}", ticket.id());
    }
    Ok(())
}

fn cmd_query(storage: &Storage, filter: Option<String>) -> Result<()> {
    ensure_init(storage)?;

    let tickets = storage.load_all_with_archived()?;

    let items: Vec<_> = tickets
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id(),
                "title": t.title,
                "status": t.meta.status.to_string(),
                "priority": t.meta.priority,
                "type": t.meta.ticket_type.to_string(),
                "deps": t.meta.deps,
                "tags": t.meta.tags,
                "created": t.meta.created,
                "parent": t.meta.parent,
            })
        })
        .collect();

    let json_str = serde_json::to_string(&items)?;

    if let Some(filter) = filter {
        // Pipe through jq if filter provided
        let mut child = Command::new("jq")
            .arg(&filter)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to run jq. Is it installed?")?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(json_str.as_bytes())?;
        }

        child.wait()?;
    } else {
        println!("{}", json_str);
    }

    Ok(())
}
