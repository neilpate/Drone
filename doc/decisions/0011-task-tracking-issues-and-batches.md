# ADR 0011 — Task tracking: GitHub Issues as canonical backlog, batched filing

- **Status:** Accepted
- **Date:** 2026-05-24
- **Related:** [ADR 0007](0007-testing-and-ci-strategy.md), closed issue [#2](https://github.com/neilpate/Drone/issues/2)

## Context

By 2026-05-24 the project has accumulated enough deferred work that "where do TODOs live" has become a real question. The candidates that surfaced:

- Inline `// TODO:` comments in source.
- A `TODO.md` at the repo root.
- A "Next session" / "Next open questions" section inside `AGENTS.md`.
- GitHub Issues.
- A GitHub Projects v2 kanban board, optionally with custom fields and automations.

Three of the four wrong answers had already been tried in some form. `AGENTS.md` "Next session" grew during the first hardware-bring-up session and became a stale duplicate of conversational state within hours. Inline `// TODO:` is fine for hyper-local notes but loses cross-cutting context. A `TODO.md` competes with the issue tracker for authority and loses every time.

The remaining question was the relationship between **issues** and **a Projects board** — and, just as important, **how aggressively to file work upfront**.

A separate failure mode worth naming: backlog inflation. Project-management dogma encourages capturing every conceivable future task so "nothing is lost". For a one-person hobby project on a multi-year horizon, an exhaustively pre-populated backlog goes stale faster than it gets worked, becomes a source of guilt rather than progress, and crowds out the few items that actually matter this week.

The author explicitly does not want Scrum, sprints, or time-boxed iterations — the dislike is for the ceremony and the time discipline, not for the underlying idea of working in coherent batches.

## Decision

### 1. GitHub Issues are the canonical backlog

Every piece of work that is worth capturing **outside of an inline `// TODO:`** is filed as an issue in `neilpate/Drone`. This includes:

- Features to implement.
- Bugs to fix.
- Refactors to do.
- Documentation / ADRs to write.
- Investigation tasks ("scope X", "decide between Y and Z").

Issues are the *only* place this kind of work lives. `AGENTS.md`, chat scrollback, source comments, sticky notes, and ad-hoc lists are not substitutes.

Closing an issue is the record that the work is done. The closing comment captures the resolution where it can be re-read in context, not in a commit message detached from the conversation that produced the decision.

### 2. Projects v2 board (#5) is a kanban *view*, not a separate backlog

The Projects v2 board at <https://github.com/users/neilpate/projects/5> exists to make the current working set visually trackable (Todo / In Progress / Done columns). It holds **a subset of issues**, manually curated.

Implications:

- Issues are not auto-added to the board. The "Auto-add to project" workflow is deliberately **not** enabled. Friction is the curation mechanism: items reach the board because they are actively being worked or actively next, not because they exist.
- No custom Project fields. Phase and area are expressed via labels, which work on issues, PRs and project items equally. Duplicating the same dimension into Project fields would create two truths.
- The default workflows that *are* enabled (closing an issue → Status=Done on the board, Status=Done → close the issue, etc.) keep the board honest without manual upkeep.

### 3. Labels are the cross-cutting taxonomy

Issues are sliced by **labels**, not by folders, milestones or Project fields. The seed taxonomy:

- **Phase**: `phase-1`, `phase-2`, ..., `phase-5`. One per issue; identifies *when* the work belongs in the milestone plan ([00-vision.md](../00-vision.md)).
- **Area**: `area:firmware`, `area:hardware`, `area:doc`, `area:tooling`. One or more per issue; identifies *what part of the system* is touched.
- Plus the GitHub defaults (`bug`, `enhancement`, `documentation`, ...).

Labels are added when they earn their keep. A label nobody filters on is noise. New labels are created when an issue genuinely doesn't fit any existing one — not pre-emptively.

### 4. Batched filing, not upfront enumeration

The backlog is **not** populated up-front for the whole project. The discipline is:

- Pick the next coherent chunk of work — a small handful of related issues (typically 2–6) that together push the system to a meaningful next state.
- File those issues. Add them to the board (Todo).
- Work them.
- When the batch is done (board is empty or near-empty), repeat: pick the next chunk, file, work.

This is sprint-shaped without being sprint-timed. The batch size is constrained by "how much can be reasoned about in one sitting", not by a calendar. There is no commitment, no review ceremony, no retrospective. The board being empty is the only signal that triggers the next planning conversation.

Things to **avoid**:

- Capturing every speculative future task "so we don't forget it". Speculative work goes in chat, ADRs (if it's a real design question), or simply in the author's head. If it matters, it will resurface when the next batch is being planned.
- Long-lived issues that drift across phases. An issue that has been open for a month without progress is a smell — either close it as "not now", break it down, or fold it into the current batch.
- Mixing batches. The board should reflect *one* working theme at a time. Two parallel themes are a sign the batch was too ambitious.

### 5. `AGENTS.md` keeps narrative context only

`AGENTS.md` is no longer a TODO tracker. It keeps:

- High-signal narrative state (vision pointers, scope guardrail, working style, glossary).
- The decisions log (an index of ADRs, not a TODO list).
- The "Next open questions" section, which is for **open design questions awaiting a decision**, not for "things to do". An open question becomes an ADR when answered; it becomes an issue when it turns into actionable work.
- A pointer to the issue tracker and the board, so a stranger landing in cold knows where the backlog actually lives.

## Consequences

**Positive.**

- One source of truth for work. Anyone (or any future assistant) reading the repo learns the convention from `AGENTS.md` and finds the live state under `/issues` and `/projects/5`.
- The board reflects current focus, not exhaustive intent. Empty columns are normal, not a failure state.
- Decisions and resolutions live attached to the work that produced them (issue + closing comment), not in distant commit messages.
- The author's stated dislike of Scrum is respected: no time-boxes, no ceremony, no velocity tracking.

**Negative.**

- Filing an issue is a small overhead. The compensating discipline is to **not** file issues for trivia — inline `// TODO:` is the right tool for "rename this variable when we touch this code next".
- Curating board membership manually is more work than auto-add. Accepted on purpose: the friction is the feature.
- Cross-project visibility is weaker than a single huge backlog. Accepted because for a one-person project, "visibility" mostly means "what should I work on next", and the board answers that.

**Operational notes.**

- A new issue: `gh issue create --repo neilpate/Drone --title "..." --body-file <file> --label "<labels>"`. Body in a file avoids PowerShell here-string fragility with `gh`.
- Add to board: `gh project item-add 5 --owner neilpate --url <issue-url>`.
- Set status: needs the field IDs (`gh project field-list 5 --owner neilpate --format json`) and `gh project item-edit ...`. A small wrapper script may be worth writing if the friction becomes real — see `area:tooling`.
- Closing an issue with a substantive resolution: `gh issue close N --repo neilpate/Drone --comment (Get-Content -Raw <file>.md)`.

## Alternatives considered

**`TODO.md` at repo root.** Rejected. Creates a second source of truth in conflict with the issue tracker; goes stale; not addressable from outside the workspace.

**A "Next session" section in `AGENTS.md`.** Tried (briefly, 2026-05-23). Rejected (2026-05-24, this ADR). Duplicates the issue list, immediately gets out of sync with chat, and pollutes a file whose job is narrative.

**Issues only, no Projects board.** Viable. Rejected because a visual kanban genuinely helps see "what's now vs. what's queued vs. what's done" at a glance, especially when work spans several days. The board's cost (one manual `item-add` per tracked issue) is low.

**Projects board with full custom-field schema (`Phase`, `Area`, `Priority`, `Estimate`, ...) and auto-add automations.** Rejected as over-engineering for a one-person project. Custom fields duplicate labels; auto-add eliminates the curation that the board's value depends on. Reconsider if the project ever grows past one contributor.

**Pre-populate the full project backlog up front.** Rejected. Backlog inflation is a known failure mode; speculative work is not work.

**Time-boxed sprints / Scrum.** Rejected by user preference. The batched-but-not-timed flow described above captures the useful part (working in coherent chunks) without the ceremony.

## References

- Closed issue [#2](https://github.com/neilpate/Drone/issues/2) — original scoping conversation and decision.
- [AGENTS.md](../../AGENTS.md) — "Backlog and task tracking" section points here.
- Project board: <https://github.com/users/neilpate/projects/5>.
