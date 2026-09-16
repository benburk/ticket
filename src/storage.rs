use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result};
use fs2::FileExt;

use crate::{find_cycles, Frontmatter, Note, Status, Ticket};

const TICKETS_DIR: &str = ".tickets";
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// An explicit ticket store. It never changes the process working directory.
pub struct Store {
    tickets_dir: PathBuf,
}

/// The former public name, retained for downstream source compatibility.
pub type Storage = Store;

impl Default for Store {
    fn default() -> Self {
        Self::discover(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl Store {
    /// Open the `.tickets` directory directly beneath a repository root.
    pub fn open(repo: impl AsRef<Path>) -> Self {
        Self {
            tickets_dir: repo.as_ref().join(TICKETS_DIR),
        }
    }

    /// Find the nearest store at or above `start`, falling back to `start`.
    pub fn discover(start: impl AsRef<Path>) -> Self {
        if let Some(override_dir) = std::env::var_os("TICKETS_DIR") {
            return Self {
                tickets_dir: PathBuf::from(override_dir),
            };
        }
        let start = start.as_ref();
        let start = if start.is_dir() {
            start.to_path_buf()
        } else {
            start.parent().unwrap_or(start).to_path_buf()
        };
        let mut current = start.clone();
        loop {
            let candidate = current.join(TICKETS_DIR);
            if candidate.is_dir() {
                return Self {
                    tickets_dir: candidate,
                };
            }
            if !current.pop() {
                break;
            }
        }
        Self::open(start)
    }

    /// Discover a store from the current directory.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn directory(&self) -> &Path {
        &self.tickets_dir
    }

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.tickets_dir)
            .with_context(|| format!("create {}", self.tickets_dir.display()))
    }

    pub fn is_initialized(&self) -> bool {
        self.tickets_dir.is_dir()
    }

    fn require_initialized(&self) -> Result<()> {
        anyhow::ensure!(
            self.is_initialized(),
            "ticket tracking is not initialized at {} (run `tk init`)",
            self.tickets_dir.display()
        );
        Ok(())
    }

    pub fn ticket_path(&self, id: &str) -> PathBuf {
        self.tickets_dir.join(format!("{id}.md"))
    }

    pub fn valid_id(id: &str) -> bool {
        id.strip_prefix("tk-").is_some_and(|suffix| {
            (4..=16).contains(&suffix.len()) && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    }

    /// Parse one complete ticket document.
    pub fn parse_ticket(content: &str) -> Result<Ticket> {
        let content = content.trim();
        let rest = content
            .strip_prefix("---")
            .context("invalid ticket format: missing YAML frontmatter")?;
        let end = rest
            .find("\n---")
            .context("invalid ticket format: unclosed frontmatter")?;
        let meta: Frontmatter =
            serde_yaml::from_str(rest[..end].trim()).context("failed to parse YAML frontmatter")?;
        anyhow::ensure!(Self::valid_id(&meta.id), "invalid ticket id `{}`", meta.id);

        let after = end + 4;
        let markdown = rest.get(after..).unwrap_or_default().trim();
        let (title, body) = Self::extract_title(markdown);
        anyhow::ensure!(title != "Untitled", "ticket has no `# Title` heading");
        Ok(Ticket {
            meta,
            title,
            body: body.to_string(),
        })
    }

    /// Extract the first leading level-one heading and the body after it.
    pub fn extract_title(body: &str) -> (String, &str) {
        for line in body.lines() {
            let trimmed = line.trim();
            if let Some(title) = trimmed.strip_prefix("# ") {
                let after = body.find(line).unwrap_or_default() + line.len();
                return (
                    title.trim().to_string(),
                    body[after..].trim_start_matches('\n'),
                );
            }
            if !trimmed.is_empty() {
                break;
            }
        }
        ("Untitled".to_string(), body)
    }

    pub fn serialize_ticket(ticket: &Ticket) -> Result<String> {
        anyhow::ensure!(
            Self::valid_id(ticket.id()),
            "invalid ticket id `{}`",
            ticket.id()
        );
        anyhow::ensure!(
            !ticket.title.trim().is_empty(),
            "ticket title cannot be empty"
        );
        let yaml = serde_yaml::to_string(&ticket.meta)?;
        let mut content = format!("---\n{yaml}---\n\n# {}\n", ticket.title.trim());
        if !ticket.body.is_empty() {
            content.push('\n');
            content.push_str(&ticket.body);
            if !ticket.body.ends_with('\n') {
                content.push('\n');
            }
        }
        Ok(content)
    }

    pub fn load(&self, id: &str) -> Result<Option<Ticket>> {
        self.require_initialized()?;
        let path = self.ticket_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let content =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let ticket =
            Self::parse_ticket(&content).with_context(|| format!("parse {}", path.display()))?;
        anyhow::ensure!(
            ticket.id() == id,
            "{} contains id `{}`",
            path.display(),
            ticket.id()
        );
        Ok(Some(ticket))
    }

    /// Load and validate the complete graph. A malformed file never disappears.
    pub fn load_all(&self) -> Result<Vec<Ticket>> {
        self.require_initialized()?;
        let mut paths: Vec<PathBuf> = fs::read_dir(&self.tickets_dir)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .collect();
        paths.sort();

        let mut tickets = Vec::with_capacity(paths.len());
        for path in paths {
            let content =
                fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
            let ticket = Self::parse_ticket(&content)
                .with_context(|| format!("parse {}", path.display()))?;
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            anyhow::ensure!(
                stem == ticket.id(),
                "{} contains id `{}`",
                path.display(),
                ticket.id()
            );
            anyhow::ensure!(
                !tickets
                    .iter()
                    .any(|existing: &Ticket| existing.id() == ticket.id()),
                "duplicate ticket id `{}`",
                ticket.id()
            );
            tickets.push(ticket);
        }
        for ticket in &tickets {
            for dependency in &ticket.meta.deps {
                anyhow::ensure!(
                    tickets.iter().any(|candidate| candidate.id() == dependency),
                    "{} depends on missing ticket `{dependency}`",
                    ticket.id()
                );
            }
        }
        tickets.sort_by_key(|ticket| ticket.meta.created);
        Ok(tickets)
    }

    /// Atomically replace one ticket while holding the store lock.
    pub fn save(&self, ticket: &Ticket) -> Result<()> {
        self.require_initialized()?;
        let _lock = self.lock()?;
        self.save_unlocked(ticket)
    }

    fn save_unlocked(&self, ticket: &Ticket) -> Result<()> {
        let content = Self::serialize_ticket(ticket)?;
        let path = self.ticket_path(ticket.id());
        let serial = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let temporary = self.tickets_dir.join(format!(
            ".{}.{}.{}.tmp",
            ticket.id(),
            std::process::id(),
            serial
        ));
        let result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.with_context(|| format!("write {}", path.display()))
    }

    fn lock(&self) -> Result<File> {
        let directory = File::open(&self.tickets_dir)?;
        directory.lock_exclusive()?;
        Ok(directory)
    }

    pub fn find_by_prefix(&self, prefix: &str) -> Result<Option<Ticket>> {
        let tickets = self.load_all()?;
        resolve_prefix(&tickets, prefix).map(|ticket| ticket.cloned())
    }

    pub fn all_ids(&self) -> Result<Vec<String>> {
        Ok(self
            .load_all()?
            .into_iter()
            .map(|ticket| ticket.meta.id)
            .collect())
    }

    /// Create a ticket in one locked read-modify-write operation.
    pub fn create(&self, title: String, body: String, tags: Vec<String>) -> Result<Ticket> {
        self.require_initialized()?;
        anyhow::ensure!(!title.trim().is_empty(), "ticket title cannot be empty");
        let _lock = self.lock()?;
        let tickets = self.load_all()?;
        let ids: Vec<String> = tickets
            .iter()
            .map(|ticket| ticket.meta.id.clone())
            .collect();
        let mut ticket = Ticket::new(crate::id::generate(&ids), title.trim().to_string());
        ticket.body = body.trim().to_string();
        ticket.meta.tags = clean_values(tags);
        self.save_unlocked(&ticket)?;
        Ok(ticket)
    }

    /// Replace user-editable content while preserving timestamps and extensions.
    pub fn replace(
        &self,
        id: &str,
        title: String,
        body: String,
        tags: Vec<String>,
        deps: Vec<String>,
    ) -> Result<Ticket> {
        self.mutate(id, |ticket, tickets| {
            anyhow::ensure!(!title.trim().is_empty(), "ticket title cannot be empty");
            let deps = clean_values(deps);
            anyhow::ensure!(
                deps.iter().all(|dependency| dependency != ticket.id()),
                "a ticket cannot depend on itself"
            );
            for dependency in &deps {
                anyhow::ensure!(
                    tickets.iter().any(|candidate| candidate.id() == dependency),
                    "dependency `{dependency}` not found"
                );
            }
            ticket.title = title.trim().to_string();
            ticket.body = body.trim().to_string();
            ticket.meta.tags = clean_values(tags);
            ticket.meta.deps = deps;
            ticket.touch();
            let mut graph: Vec<Ticket> = tickets
                .iter()
                .filter(|candidate| candidate.id() != ticket.id())
                .cloned()
                .collect();
            graph.push(ticket.clone());
            anyhow::ensure!(find_cycles(&graph).is_empty(), "dependencies form a cycle");
            Ok(())
        })
    }

    pub fn close(&self, id: &str) -> Result<Ticket> {
        self.mutate(id, |ticket, _| {
            ticket.set_status(Status::Closed);
            Ok(())
        })
    }

    pub fn reopen(&self, id: &str) -> Result<Ticket> {
        self.mutate(id, |ticket, _| {
            ticket.set_status(Status::Open);
            Ok(())
        })
    }

    pub fn append_note(&self, id: &str, content: String) -> Result<Ticket> {
        anyhow::ensure!(!content.trim().is_empty(), "note cannot be empty");
        self.mutate(id, |ticket, _| {
            if !ticket.body.is_empty() {
                ticket.body.push_str("\n\n");
            }
            ticket
                .body
                .push_str(&Note::new(content.trim().to_string()).format());
            ticket.touch();
            Ok(())
        })
    }

    /// Delete a ticket and remove its ID from dependents under one store lock.
    pub fn delete(&self, prefix: &str) -> Result<Ticket> {
        self.require_initialized()?;
        let _lock = self.lock()?;
        let tickets = self.load_all()?;
        let ticket = resolve_prefix(&tickets, prefix)?
            .context(format!("ticket `{prefix}` not found"))?
            .clone();
        for mut dependent in tickets
            .into_iter()
            .filter(|candidate| candidate.id() != ticket.id())
        {
            let before = dependent.meta.deps.len();
            dependent
                .meta
                .deps
                .retain(|dependency| dependency != ticket.id());
            if dependent.meta.deps.len() != before {
                dependent.touch();
                self.save_unlocked(&dependent)?;
            }
        }
        fs::remove_file(self.ticket_path(ticket.id()))?;
        Ok(ticket)
    }

    fn mutate(
        &self,
        prefix: &str,
        change: impl FnOnce(&mut Ticket, &[Ticket]) -> Result<()>,
    ) -> Result<Ticket> {
        self.require_initialized()?;
        let _lock = self.lock()?;
        let tickets = self.load_all()?;
        let id = resolve_prefix(&tickets, prefix)?
            .context(format!("ticket `{prefix}` not found"))?
            .id()
            .to_string();
        let mut ticket = tickets
            .iter()
            .find(|ticket| ticket.id() == id)
            .cloned()
            .expect("resolved ticket remains in the loaded set");
        change(&mut ticket, &tickets)?;
        self.save_unlocked(&ticket)?;
        Ok(ticket)
    }
}

fn clean_values(values: Vec<String>) -> Vec<String> {
    let mut clean = Vec::new();
    for value in values {
        let value = value.trim();
        if !value.is_empty() && !clean.iter().any(|existing| existing == value) {
            clean.push(value.to_string());
        }
    }
    clean
}

fn resolve_prefix<'a>(tickets: &'a [Ticket], prefix: &str) -> Result<Option<&'a Ticket>> {
    if let Some(ticket) = tickets.iter().find(|ticket| ticket.id() == prefix) {
        return Ok(Some(ticket));
    }
    let matches: Vec<&Ticket> = tickets
        .iter()
        .filter(|ticket| ticket.id().starts_with(prefix))
        .collect();
    match matches.as_slice() {
        [] => Ok(None),
        [ticket] => Ok(Some(*ticket)),
        _ => anyhow::bail!(
            "ambiguous prefix `{prefix}`: matches {} tickets; use the full id",
            matches.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ticket-{name}-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn reads_do_not_initialize_a_store() {
        let root = fixture("no-init");
        let store = Store::open(&root);
        assert!(store.load_all().is_err());
        assert!(!root.join(TICKETS_DIR).exists());
    }

    #[test]
    fn creates_and_atomically_replaces_a_ticket() {
        let root = fixture("create");
        let store = Store::open(&root);
        store.init().unwrap();
        let mut ticket = store
            .create("A title".into(), "A body".into(), vec!["ui".into()])
            .unwrap();
        ticket.title = "A safer title".into();
        ticket.touch();
        store.save(&ticket).unwrap();
        assert_eq!(store.load_all().unwrap()[0].title, "A safer title");
        assert!(fs::read_dir(store.directory())
            .unwrap()
            .flatten()
            .all(|entry| !entry.file_name().to_string_lossy().ends_with(".tmp")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lifecycle_and_edits_share_the_store_rules() {
        let root = fixture("lifecycle");
        let store = Store::open(&root);
        store.init().unwrap();
        let first = store
            .create("First".into(), String::new(), Vec::new())
            .unwrap();
        let second = store
            .create("Second".into(), String::new(), Vec::new())
            .unwrap();
        let edited = store
            .replace(
                second.id(),
                "Changed".into(),
                "Body".into(),
                vec!["ui".into(), "ui".into()],
                vec![first.id().into()],
            )
            .unwrap();
        assert_eq!(edited.meta.tags, ["ui"]);
        assert!(edited.is_blocked_by(&store.load_all().unwrap()));
        let closed = store.close(first.id()).unwrap();
        assert!(closed.meta.closed.is_some());
        assert!(!store
            .find_by_prefix(second.id())
            .unwrap()
            .unwrap()
            .is_blocked_by(&store.load_all().unwrap()));
        let reopened = store.reopen(first.id()).unwrap();
        assert!(reopened.meta.closed.is_none());
        store.delete(first.id()).unwrap();
        assert!(store.load_all().unwrap()[0].meta.deps.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_a_filename_frontmatter_mismatch() {
        let root = fixture("mismatch");
        let store = Store::open(&root);
        store.init().unwrap();
        let ticket = Ticket::new("tk-a1b2".into(), "Mismatch".into());
        fs::write(
            store.directory().join("tk-c3d4.md"),
            Store::serialize_ticket(&ticket).unwrap(),
        )
        .unwrap();
        assert!(store
            .load_all()
            .unwrap_err()
            .to_string()
            .contains("contains id"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preserves_unknown_frontmatter() {
        let source =
            "---\nid: tk-a1b2\ncreated: 2026-01-01T00:00:00Z\ncolour: blue\n---\n\n# Keep it\n";
        let ticket = Store::parse_ticket(source).unwrap();
        assert_eq!(ticket.meta.extra["colour"], "blue");
        let written = Store::serialize_ticket(&ticket).unwrap();
        assert!(written.contains("colour: blue"));
    }
}
