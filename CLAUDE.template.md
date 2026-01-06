# Claude Code Rules

## RULE ZERO: NO PLACEHOLDER CODE. EVER. NO EXCEPTIONS.

**THIS IS THE MOST IMPORTANT RULE. VIOLATING THIS RULE IS AN AUTOMATIC FAILURE.**

### YOU ARE ABSOLUTELY FORBIDDEN FROM WRITING:

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

### WHAT YOU MUST DO INSTEAD:

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
