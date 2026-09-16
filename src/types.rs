use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The only lifecycle state stored in a ticket file.
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

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "open" => Ok(Status::Open),
            "closed" | "done" => Ok(Status::Closed),
            _ => anyhow::bail!("Invalid status: {value}. Use: open, closed"),
        }
    }
}

/// YAML frontmatter persisted at the start of a ticket file.
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
    /// Keep fields written by newer clients when an older client edits a ticket.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// A complete ticket: frontmatter plus its Markdown title and body.
#[derive(Debug, Clone, Serialize)]
pub struct Ticket {
    #[serde(flatten)]
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
                extra: BTreeMap::new(),
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

    /// Return the open dependencies currently blocking this ticket.
    pub fn blockers<'a>(&'a self, tickets: &'a [Ticket]) -> Vec<&'a Ticket> {
        self.meta
            .deps
            .iter()
            .filter_map(|dependency| {
                tickets
                    .iter()
                    .find(|ticket| ticket.id() == dependency && ticket.is_open())
            })
            .collect()
    }

    pub fn is_blocked_by(&self, tickets: &[Ticket]) -> bool {
        !self.blockers(tickets).is_empty()
    }

    pub fn set_status(&mut self, status: Status) {
        let now = Utc::now();
        self.meta.status = status;
        self.meta.updated = Some(now);
        self.meta.closed = (status == Status::Closed).then_some(now);
    }

    pub fn touch(&mut self) {
        self.meta.updated = Some(Utc::now());
    }
}

/// A timestamped progress or decision note appended to the Markdown body.
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
