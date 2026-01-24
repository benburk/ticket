use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Ticket status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Open,
    Closed,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for Status {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" | "in-progress" | "in_progress" | "inprogress" | "started" => Ok(Status::Open),
            "closed" | "done" | "archived" => Ok(Status::Closed),
            _ => anyhow::bail!("Invalid status: {}. Use: open, closed", s),
        }
    }
}

/// Ticket type (aligned with conventional commits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TicketType {
    #[default]
    Feat, // New feature (MINOR version bump)
    Fix,      // Bug fix (PATCH version bump)
    Chore,    // Maintenance, deps, no user impact
    Docs,     // Documentation only
    Refactor, // Code change, no behavior change
    Test,     // Test coverage
}

impl std::fmt::Display for TicketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketType::Feat => write!(f, "feat"),
            TicketType::Fix => write!(f, "fix"),
            TicketType::Chore => write!(f, "chore"),
            TicketType::Docs => write!(f, "docs"),
            TicketType::Refactor => write!(f, "refactor"),
            TicketType::Test => write!(f, "test"),
        }
    }
}

impl std::str::FromStr for TicketType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            // Primary types
            "feat" | "feature" => Ok(TicketType::Feat),
            "fix" | "bug" => Ok(TicketType::Fix),
            "chore" => Ok(TicketType::Chore),
            "docs" => Ok(TicketType::Docs),
            "refactor" => Ok(TicketType::Refactor),
            "test" => Ok(TicketType::Test),
            // Legacy aliases
            "task" => Ok(TicketType::Feat),
            "epic" => Ok(TicketType::Feat),
            _ => anyhow::bail!(
                "Invalid type: {}. Use: feat, fix, chore, docs, refactor, test",
                s
            ),
        }
    }
}

/// YAML frontmatter for a ticket file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub id: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<String>,
    pub created: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed: Option<DateTime<Utc>>,
    #[serde(default, rename = "type")]
    pub ticket_type: TicketType,
    #[serde(default)]
    pub priority: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// A complete ticket (frontmatter + body)
#[derive(Debug, Clone)]
pub struct Ticket {
    pub meta: Frontmatter,
    pub title: String,
    pub body: String,
}

impl Ticket {
    pub fn new(id: String, title: String) -> Self {
        Self {
            meta: Frontmatter {
                id,
                status: Status::Open,
                deps: vec![],
                created: Utc::now(),
                updated: None,
                closed: None,
                ticket_type: TicketType::Feat,
                priority: 2,
                assignee: None,
                parent: None,
                tags: vec![],
            },
            title,
            body: String::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.meta.id
    }

    pub fn is_open(&self) -> bool {
        self.meta.status == Status::Open
    }

    pub fn is_blocked_by(&self, tickets: &[Ticket]) -> bool {
        for dep_id in &self.meta.deps {
            if let Some(dep) = tickets.iter().find(|t| t.id() == dep_id) {
                if dep.is_open() {
                    return true;
                }
            }
        }
        false
    }

    pub fn touch(&mut self) {
        self.meta.updated = Some(Utc::now());
    }
}

/// A timestamped note (appended to body)
#[derive(Debug, Clone)]
pub struct Note {
    pub timestamp: DateTime<Utc>,
    pub author: Option<String>,
    pub content: String,
}

impl Note {
    pub fn new(content: String) -> Self {
        Self {
            timestamp: Utc::now(),
            author: std::env::var("USER").ok(),
            content,
        }
    }

    pub fn format(&self) -> String {
        let author = self.author.as_deref().unwrap_or("anonymous");
        format!(
            "[{} {}] {}",
            self.timestamp.format("%Y-%m-%d %H:%M"),
            author,
            self.content
        )
    }
}
