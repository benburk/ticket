use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Ticket status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Open,
    #[serde(rename = "in-progress")]
    InProgress,
    Closed,
    Archived,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::InProgress => write!(f, "in-progress"),
            Status::Closed => write!(f, "closed"),
            Status::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for Status {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Status::Open),
            "in-progress" | "in_progress" | "inprogress" | "started" => Ok(Status::InProgress),
            "closed" | "done" => Ok(Status::Closed),
            "archived" => Ok(Status::Archived),
            _ => anyhow::bail!("Invalid status: {}. Use: open, in-progress, closed", s),
        }
    }
}

/// Ticket type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TicketType {
    #[default]
    Task,
    Bug,
    Feature,
    Epic,
    Chore,
}

impl std::fmt::Display for TicketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketType::Task => write!(f, "task"),
            TicketType::Bug => write!(f, "bug"),
            TicketType::Feature => write!(f, "feature"),
            TicketType::Epic => write!(f, "epic"),
            TicketType::Chore => write!(f, "chore"),
        }
    }
}

impl std::str::FromStr for TicketType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task" => Ok(TicketType::Task),
            "bug" => Ok(TicketType::Bug),
            "feature" => Ok(TicketType::Feature),
            "epic" => Ok(TicketType::Epic),
            "chore" => Ok(TicketType::Chore),
            _ => anyhow::bail!("Invalid type: {}. Use: task, bug, feature, epic, chore", s),
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
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
                links: vec![],
                created: Utc::now(),
                updated: None,
                closed: None,
                ticket_type: TicketType::Task,
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
        matches!(self.meta.status, Status::Open | Status::InProgress)
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
