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
            // Two statuses only. In-progress isn't a status — an existing branch
            // named for the ticket signals that (see PHILOSOPHY.md).
            "open" => Ok(Status::Open),
            "closed" | "done" => Ok(Status::Closed),
            _ => anyhow::bail!("Invalid status: {}. Use: open, closed", s),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
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
                assignee: None,
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
