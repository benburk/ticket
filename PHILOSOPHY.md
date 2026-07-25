# Philosophy

## Unix

tk does one thing: track tickets as files. It doesn't manage branches, worktrees, or CI. It produces structured data that pipes to other tools.

```bash
tk query | jq '.[] | select(.priority < 2)'
tk ready | head -1 | cut -d' ' -f1 | xargs git checkout -b
```

## Files Over State

Tickets are markdown files. No database, no daemon, no sync. Git handles history, merging, and distribution. Your editor handles editing.

## Explicit Over Magic

- No automatic branch creation
- No status inference from git state
- No hidden coordination

tk records intent. You execute it.

## Git Integration

### Core Model

Separate isolation from work tracking:

```
worktree = agent workspace (long-lived)
branch   = ticket (ephemeral)
```

### Why

Agents need isolation. They run tests, bind ports, write build artifacts. Two agents in the same directory collide. Worktrees solve this.

But worktrees are expensive to recreate—build caches, node_modules, compiled artifacts. Deleting a worktree per ticket wastes this work.

Solution: worktrees persist across tickets. Branches come and go within them.

```bash
# Agent gets a worktree once
git worktree add .worktrees/agent-1 main

# Works on tickets sequentially in the same worktree
cd .worktrees/agent-1
git checkout -b tk-a1b2
# ... work, merge ...
git checkout main && git pull
git checkout -b tk-c3d4
# ... build artifacts preserved ...
```

### Coordination

```
worktree exists   → agent is active
branch exists     → ticket in progress
branch on remote  → ready for review
branch merged     → done
```

`tk ready` lists open tickets with no blocking deps. To find unclaimed tickets, filter out those with existing branches:

```bash
tk ready | while read line; do
  id=$(echo "$line" | cut -d' ' -f1)
  git branch --list "$id" | grep -q . || echo "$line"
done
```

Creating a branch claims the ticket. No locks, no status commits to main, no external coordination.

### Naming

```
branch:    <ticket-id>              # tk-a1b2
worktree:  .worktrees/<agent-name>  # .worktrees/agent-1
```

Branches named by ticket. Worktrees named by agent or purpose.

## Status

Two statuses: `open` and `closed`. That's it.

No `in-progress`—branch existence signals this. No `archived`—move files manually if you want to hide old tickets. Fewer statuses means fewer decisions and less state to synchronize.

## Dependencies

`deps` model blocking work, not related work. If A cannot start until B closes, A depends on B. Use tags for grouping.

## No Types, No Priority

Earlier versions carried a conventional-commits `type` (feat/fix/…) and a numeric `priority`. Both are gone. They failed the same test everything else here passes: **a field should only exist if something acts on it, and only if a human isn't taxed to guess it.**

`type` (feat/fix/chore/…) is the weakest part of conventional commits. In practice the choice is a dice roll—real changes are a fix *and* a refactor *and* a feature—and the type is usually redundant with the title ("fix namespaced SVG styles" already reads as a fix). Its one real payoff, driving semver + changelogs, only matters for *published packages*, and even then people prefer scope over type: knowing *where* a change landed beats knowing *what kind*. That's derivable—`git diff --stat` on the ticket's branch tells you the scope for free—so no annotation is warranted.

`priority` is a fake signal. Everything drifts to the default; you spend attention curating numbers instead of working. Real urgency is derivable from structure: a ticket that blocks others (`deps`) and has aged is high-priority without anyone setting a field.

What survives does so because it's *structural and underivable*: `deps` (blocking, gates readiness) and `tags` (grouping related work / epics—the one classification a diff can't reveal). Phase itself isn't stored at all: to-do / in-progress / done is derived from branch existence and `status`. Fewer fields, fewer decisions, less state to synchronize.

## Notes Over Comments

Notes are append-only, timestamped entries in the ticket body. They're for progress updates, decisions, and context—things that matter when you read the ticket later. Not for discussion; use PRs for that.
