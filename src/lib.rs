//! Filesystem-backed tickets for people, editors, agents, and small tools.
//!
//! This crate owns the `.tickets/*.md` format and its graph invariants. It
//! deliberately does not manage Git branches, worktrees, agents, or review.

mod id;
mod storage;
mod types;

pub use storage::{Storage, Store};
pub use types::{Frontmatter, Note, Status, Ticket};

use std::collections::{HashMap, HashSet};

/// Find dependency cycles, each represented without repeating its first node.
pub fn find_cycles(tickets: &[Ticket]) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    let mut path = Vec::new();
    let by_id: HashMap<&str, &Ticket> =
        tickets.iter().map(|ticket| (ticket.id(), ticket)).collect();

    for ticket in tickets {
        if !visited.contains(ticket.id()) {
            visit(
                ticket.id(),
                &by_id,
                &mut visited,
                &mut stack,
                &mut path,
                &mut cycles,
            );
        }
    }
    cycles
}

fn visit(
    id: &str,
    tickets: &HashMap<&str, &Ticket>,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(id.to_string());
    stack.insert(id.to_string());
    path.push(id.to_string());
    if let Some(ticket) = tickets.get(id) {
        for dependency in &ticket.meta.deps {
            if !visited.contains(dependency) {
                visit(dependency, tickets, visited, stack, path, cycles);
            } else if stack.contains(dependency) {
                if let Some(start) = path.iter().position(|candidate| candidate == dependency) {
                    cycles.push(path[start..].to_vec());
                }
            }
        }
    }
    path.pop();
    stack.remove(id);
}
