use crate::types::{Frontmatter, Ticket};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const TICKETS_DIR: &str = ".tickets";

pub struct Storage {
    tickets_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Self {
        let tickets_dir = Self::find_tickets_dir();
        Self { tickets_dir }
    }

    /// Find .tickets directory by searching current and parent directories.
    /// Falls back to ./.tickets if not found (for init).
    fn find_tickets_dir() -> PathBuf {
        // Check TICKETS_DIR env var first
        if let Ok(dir) = std::env::var("TICKETS_DIR") {
            return PathBuf::from(dir);
        }

        // Search current and parent directories
        if let Ok(mut current) = std::env::current_dir() {
            loop {
                let candidate = current.join(TICKETS_DIR);
                if candidate.is_dir() {
                    return candidate;
                }
                if !current.pop() {
                    break;
                }
            }
        }

        // Default to current directory (for init)
        PathBuf::from(TICKETS_DIR)
    }

    pub fn init(&self) -> Result<()> {
        if !self.tickets_dir.exists() {
            fs::create_dir_all(&self.tickets_dir)?;
        }
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.tickets_dir.exists()
    }

    pub fn ticket_path(&self, id: &str) -> PathBuf {
        self.tickets_dir.join(format!("{}.md", id))
    }

    /// Parse a markdown file with YAML frontmatter into a Ticket
    fn parse_ticket(content: &str) -> Result<Ticket> {
        let content = content.trim();

        // Must start with ---
        if !content.starts_with("---") {
            anyhow::bail!("Invalid ticket format: missing YAML frontmatter");
        }

        // Find the closing ---
        let rest = &content[3..];
        let end = rest
            .find("\n---")
            .context("Invalid ticket format: unclosed frontmatter")?;

        let yaml_str = &rest[..end].trim();
        let body_start = end + 4; // skip \n---
        let body = if body_start < rest.len() {
            rest[body_start..].trim()
        } else {
            ""
        };

        // Parse YAML frontmatter
        let meta: Frontmatter =
            serde_yaml::from_str(yaml_str).context("Failed to parse YAML frontmatter")?;

        // Extract title from first markdown heading
        let (title, body) = Self::extract_title(body);

        Ok(Ticket {
            meta,
            title,
            body: body.to_string(),
        })
    }

    /// Extract title from first # heading, return (title, remaining body)
    pub fn extract_title(body: &str) -> (String, &str) {
        for line in body.lines() {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix("# ") {
                let title = stripped.trim().to_string();
                // Find where this line ends and return the rest
                if let Some(pos) = body.find(line) {
                    let after = pos + line.len();
                    let rest = body[after..].trim_start_matches('\n');
                    return (title, rest);
                }
            } else if !trimmed.is_empty() {
                // Non-empty, non-heading line - no title found
                break;
            }
        }
        ("Untitled".to_string(), body)
    }

    /// Serialize a Ticket to markdown with YAML frontmatter
    fn serialize_ticket(ticket: &Ticket) -> Result<String> {
        let yaml = serde_yaml::to_string(&ticket.meta)?;
        let mut content = format!("---\n{}---\n\n# {}\n", yaml, ticket.title);

        if !ticket.body.is_empty() {
            content.push('\n');
            content.push_str(&ticket.body);
            if !ticket.body.ends_with('\n') {
                content.push('\n');
            }
        }

        Ok(content)
    }

    /// Load a single ticket by ID
    #[allow(dead_code)]
    pub fn load(&self, id: &str) -> Result<Option<Ticket>> {
        let path = self.ticket_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        Ok(Some(Self::parse_ticket(&content)?))
    }

    /// Load all tickets
    pub fn load_all(&self) -> Result<Vec<Ticket>> {
        self.load_from_dir(&self.tickets_dir)
    }

    fn load_from_dir(&self, dir: &Path) -> Result<Vec<Ticket>> {
        let mut tickets = Vec::new();

        if !dir.exists() {
            return Ok(tickets);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "md") {
                let content = fs::read_to_string(&path)?;
                match Self::parse_ticket(&content) {
                    Ok(ticket) => tickets.push(ticket),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(tickets)
    }

    /// Save a ticket
    pub fn save(&self, ticket: &Ticket) -> Result<()> {
        let path = self.ticket_path(ticket.id());
        let content = Self::serialize_ticket(ticket)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Find a ticket by ID prefix
    pub fn find_by_prefix(&self, prefix: &str) -> Result<Option<Ticket>> {
        let tickets = self.load_all()?;

        // Exact match first
        if let Some(ticket) = tickets.iter().find(|t| t.id() == prefix) {
            return Ok(Some(ticket.clone()));
        }

        // Prefix match
        let matches: Vec<_> = tickets
            .iter()
            .filter(|t| t.id().starts_with(prefix))
            .collect();

        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches[0].clone())),
            _ => anyhow::bail!(
                "Ambiguous prefix '{}': matches {} tickets. Use full ID.",
                prefix,
                matches.len()
            ),
        }
    }

    /// Get all existing ticket IDs
    pub fn all_ids(&self) -> Result<Vec<String>> {
        Ok(self
            .load_all()?
            .iter()
            .map(|t| t.id().to_string())
            .collect())
    }
}
