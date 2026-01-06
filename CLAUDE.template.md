# Claude Code Rules

## RULE ZERO — NO PLACEHOLDER CODE EVER

**THIS IS THE MOST IMPORTANT RULE. VIOLATING THIS RULE IS AN AUTOMATIC FAILURE.**

### Forbidden Patterns

- `// TODO` — **BANNED**
- `// FIXME` — **BANNED**
- `// implement later` — **BANNED**
- `// add logic here` — **BANNED**
- `pass` with no implementation — **BANNED**
- `return null` as a stub — **BANNED**
- `throw new Error("not implemented")` — **BANNED**
- Empty function bodies — **BANNED**
- Comments describing what code SHOULD do instead of ACTUAL CODE — **BANNED**
- Pseudocode instead of real code — **BANNED**
- Comments stating that 'in production code this would be implemented as X' — **BANNED**

### Required Behavior

- **IMPLEMENT THE ACTUAL CODE.** Not tomorrow. Not later. NOW.
- If you don't know how to implement something: **STOP AND ASK.** Do NOT stub it.
- If it's too complex: **BREAK IT DOWN.** Do NOT leave placeholders.
- Every function you write MUST BE COMPLETE AND WORKING.

---

## ⚠️ MANDATORY RULES

### 1. NATIVE TOOLS — NOT CLI

**THESE COMMANDS ARE BANNED. DO NOT USE THEM:**

`cat` `head` `tail` `less` `more` → **Use Read tool**
`grep` `rg` `ag` `ack` → **Use Grep tool**
`find` `ls` `fd` `tree` → **Use Glob tool**
`sed` `awk` `perl -pe` → **Use Edit tool**
`echo >` `cat <<EOF` `printf >` `tee` → **Use Write tool**
`curl` `wget` → **Use WebFetch tool**
`git` → **You will NEVER use git, in any form!**

**ONLY EXCEPTIONS:** actual build/runtime tools with no native equivalent.

### 2. QUALITY GATES — MANDATORY BEFORE "DONE"

You are NOT done until ALL of these pass:

- ✅ Zero IDE diagnostics (errors AND warnings)
- ✅ All tests pass
- ✅ No linting errors
- ✅ You have thoroughly reviewed your work, both from a correctness AND a security standpoint and found nothing wrong! Follow project specific guidance, if available, to do so.

**SINGLE-PASS COMPLETION IS FORBIDDEN.** Run the verification loop. Actually check.

### 3. SUBAGENTS ARE BLIND — YOU **MUST** PASS CONTEXT

Subagents DO NOT SEE BY DEFAULT:

- This CLAUDE.md file
- The project CLAUDE.md file
- Your conversation history
- Files you've read

**YOU MUST INCLUDE IN EVERY SUBAGENT PROMPT:**

1. ALL CLAUDE.md content. This is non-negotiable. COPY IT ALL, THEY NEED TO KNOW THE RULES!
2. Complete task context
3. Expected output format

### 4. CONTEXT LIMIT — 85% = STOP -- ALWAYS

At 85% context: STOP. Write handoff to CLAUDE.md. Do NOT start new tasks. Do NOT "finish quickly." Let the user know you are at context limit and need to hand off.

### 5. SESSION STATE

UPDATE CLAUDE.md FREQUENTLY during work updating your session state and progress. **DELETE THESE UPDATES** when task fully completes, REPLACING THEM with a short summary of what was done.

### 6. DOCUMENT MISTAKES AND REMEDIATIONS

If you make a mistake, and the user points it out OR you discover it yourself while reviewing, you MUST document:

1. What the mistake was
2. What you should have done instead
3. How you fixed it

as soon as the mistake is identified. Keep it succinct as possible in CLAUDE.md under a "MISTAKES TO AVOID" section. NEVER delete content in this section. ALWAYS append to it.

### 6. PLANS LOCATION

NEVER USE `~/.claude/plans`. We use `./doc/plans/PLAN-XXXX-description.md`. You CAN create the `doc/plans` directory if it doesn't exist and you CAN create plan files there, even in plan mode. This rule supersedes any previous/default behaviour you were following.

### 7. DEPENDENCIES — PACKAGE MANAGERS ONLY

**NEVER** manually edit package.json, Cargo.toml, pyproject.toml, go.mod, etc.
**ALWAYS** use: `npm install`, `cargo add`, `uv add`, `go get`

---

## Subagents and Skills

You MUST always DELEGATE ALL WORK to subagents. This is non-negotiable. You MUST NOT do any work yourself. Spawn multiple agents AT ONCE whenever possible, and DISTRIBUTE the work to them.

## Code Quality

**Size Limits:** Files 400 lines | Functions 50 lines | Classes 300 lines — IMPORTANT: Refactor code if exceeded

AS SOON AS YOU READ THESE RULES ACKNOWLEDGE THAT YOU UNDERSTAND THEM AND WILL FOLLOW THEM STRICTLY.

---

## Work Orchestration (Flux)

This section enables self-propelling agents that survive context exhaustion and crashes.

### The Signal Principle

> **"If you have a signal, answer it."**

On session start:

1. Check `.work/signals/` for pending work matching your role
2. If signal exists → read it, load context files listed in "Context Restoration", execute immediately
3. If no signal → ask what to do

### The Clear > Compact Principle

> **"Don't fight lossy compression. Externalize state and start fresh."**

At 75% context usage (Yellow zone):

1. Create handoff in `.work/handoffs/` with structured format (see below)
2. Update your signal with next steps
3. Update runner status to `context-exhausted` in `.work/runners/<id>.md`
4. Clear context (NOT compact)
5. Fresh session loads signal + handoff

**Context Thresholds:**

| Level  | Usage  | Action                                |
| ------ | ------ | ------------------------------------- |
| Green  | < 60%  | Normal operation                      |
| Yellow | 60-74% | Warning - consider handoff soon       |
| Red    | ≥ 75%  | Critical - create handoff immediately |

### Before Ending ANY Session

1. Update runner status in `.work/runners/<id>.md`
2. If work remains:
   - Write handoff to `.work/handoffs/YYYY-MM-DD-description.md`
   - Include: Goals, completed work, decisions made, file:line references, next steps
   - Update signal with next steps
3. If blocked:
   - Document blocker in track file `.work/tracks/<track>.md`
   - Update signal with blocker details

### Self-Identification Mechanism

When you start a session:

1. Scan `.work/signals/` for files with pending work
2. Match role field to your capabilities
3. One match → you ARE that runner, execute the signal
4. Multiple matches → ask user which runner to assume
5. No matches → ask user (create new runner? wait for assignment?)

### file:line References (CRITICAL)

**ALWAYS** use specific references like `src/auth.ts:45-120` instead of vague descriptions.

Good: "Implement token refresh in `src/middleware/auth.ts:121+`"
Bad: "Continue working on the auth middleware"

This enables precise context restoration when resuming work.

### Handoff File Format

When creating handoffs, use this structure:

```markdown
# Handoff: [Brief Description]

## Metadata

- **Date**: YYYY-MM-DD
- **From**: [runner-id] ([role])
- **To**: [runner-id] (next session) or any [role]
- **Track**: [track-id]
- **Context**: [X]% (approaching threshold)

## Goals (What We're Building)

[1-2 sentences describing the overall goal]

## Completed Work

- [Specific accomplishment with file:line ref]
- [Another accomplishment]

## Key Decisions Made

| Decision | Rationale |
| -------- | --------- |
| [Choice] | [Why]     |

## Current State

- **Branch**: [branch-name]
- **Tests**: [status]
- **Files Modified**: [list with paths]

## Next Steps (Prioritized)

1. [Most important task] in `path/file.ext:line+`
2. [Second task]
3. [Third task]

## Learnings / Patterns Identified

- [Useful insight for future work]
```

### Signal File Format

Signals tell a runner what to do. Format:

```markdown
# Signal: [runner-id]

## Target

- **Runner**: [runner-id]
- **Role**: [role-type]
- **Track**: [track-id]

## Signal

[signal-type]: [brief description]

## Work

[Detailed description of what needs to be done]

### Immediate Tasks

1. [First task]
2. [Second task]
3. [Third task]

### Context Restoration (file:line references)

- `.work/tracks/[track].md` - Track overview
- `.work/handoffs/[date]-[desc].md` - Previous handoff
- `src/path/file.ext:line-range` - Relevant code

### Acceptance Criteria

- [ ] [Criterion 1]
- [ ] [Criterion 2]
```

### Team Workflow References (Optional)

If your team has workflow documentation, reference it here:

- Feature development: [CONTRIBUTING.md or wiki link]
- Bug fixes: [process doc or link]
- Code review: [PR guidelines]

Note: External references (Notion, Linear, Confluence) may require MCP servers.
