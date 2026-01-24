use crate::types::{Frontmatter, Ticket};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const TICKETS_DIR: &str = ".tickets";
const ARCHIVE_DIR: &str = "archive";

pub struct Storage {
    tickets_dir: PathBuf,
    archive_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Self {
        let tickets_dir = PathBuf::from(TICKETS_DIR);
        let archive_dir = tickets_dir.join(ARCHIVE_DIR);
        Self {
            tickets_dir,
            archive_dir,
        }
    }

    pub fn init(&self) -> Result<()> {
        if !self.tickets_dir.exists() {
            fs::create_dir_all(&self.tickets_dir)?;
        }
        if !self.archive_dir.exists() {
            fs::create_dir_all(&self.archive_dir)?;
        }
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.tickets_dir.exists()
    }

    fn ticket_path(&self, id: &str) -> PathBuf {
        self.tickets_dir.join(format!("{}.md", id))
    }

    fn archive_path(&self, id: &str) -> PathBuf {
        self.archive_dir.join(format!("{}.md", id))
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
            if trimmed.starts_with("# ") {
                let title = trimmed[2..].trim().to_string();
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
            // Check archive
            let archive_path = self.archive_path(id);
            if archive_path.exists() {
                let content = fs::read_to_string(&archive_path)?;
                return Ok(Some(Self::parse_ticket(&content)?));
            }
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        Ok(Some(Self::parse_ticket(&content)?))
    }

    /// Load all tickets (not archived)
    pub fn load_all(&self) -> Result<Vec<Ticket>> {
        self.load_from_dir(&self.tickets_dir, false)
    }

    /// Load all tickets including archived
    pub fn load_all_with_archived(&self) -> Result<Vec<Ticket>> {
        let mut tickets = self.load_from_dir(&self.tickets_dir, false)?;
        if self.archive_dir.exists() {
            tickets.extend(self.load_from_dir(&self.archive_dir, true)?);
        }
        Ok(tickets)
    }

    fn load_from_dir(&self, dir: &Path, _is_archive: bool) -> Result<Vec<Ticket>> {
        let mut tickets = Vec::new();

        if !dir.exists() {
            return Ok(tickets);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "md") {
                // Skip archive directory when reading from tickets_dir
                if path.file_name().map_or(false, |n| n == "archive") {
                    continue;
                }

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

    /// Archive a ticket (move to archive directory)
    pub fn archive(&self, id: &str) -> Result<()> {
        let src = self.ticket_path(id);
        let dst = self.archive_path(id);

        if !src.exists() {
            anyhow::bail!("Ticket {} not found", id);
        }

        fs::rename(&src, &dst)?;
        Ok(())
    }

    /// Unarchive a ticket (move back from archive)
    pub fn unarchive(&self, id: &str) -> Result<()> {
        let src = self.archive_path(id);
        let dst = self.ticket_path(id);

        if !src.exists() {
            anyhow::bail!("Archived ticket {} not found", id);
        }

        fs::rename(&src, &dst)?;
        Ok(())
    }

    /// Delete a ticket permanently
    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.ticket_path(id);
        if path.exists() {
            fs::remove_file(&path)?;
            return Ok(());
        }

        let archive_path = self.archive_path(id);
        if archive_path.exists() {
            fs::remove_file(&archive_path)?;
            return Ok(());
        }

        anyhow::bail!("Ticket {} not found", id);
    }

    /// Find a ticket by ID prefix
    pub fn find_by_prefix(&self, prefix: &str) -> Result<Option<Ticket>> {
        let tickets = self.load_all_with_archived()?;

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
