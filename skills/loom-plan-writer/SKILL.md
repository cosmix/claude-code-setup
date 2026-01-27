---
name: loom-plan-writer
description: REQUIRED skill for creating Loom execution plans. Designs DAG-based plans with mandatory knowledge-bootstrap and integration-verify bookends, parallel subagent execution within stages, and concurrent worktree stages for maximum throughput. Trigger keywords: loom, plan, stage, worktree, orchestration, parallel execution, parallel stages, concurrent execution, knowledge-bootstrap, integration-verify, acceptance criteria, signal, handoff, execution graph, dag, dependencies, loom plan, create plan, write plan, execution plan, orchestration plan, stage dependencies, parallel subagents, functional verification, wiring verification, smoke test.
allowed-tools: Read, Grep, Glob, Write, Edit
---

# Loom Plan Writer

## Overview

**THIS IS THE REQUIRED SKILL FOR CREATING LOOM EXECUTION PLANS.**

When any agent needs to create a plan for Loom orchestration, this skill MUST be invoked. This skill ensures:

- Correct plan structure with mandatory `knowledge-bootstrap` (first) and `integration-verify` (last) stages
- Proper YAML metadata formatting (3 backticks, no nested code fences)
- Parallelization strategy (subagents within stages FIRST, separate stages SECOND)
- Functional verification requirements (tests passing ≠ feature working)
- Alignment with all CLAUDE.md rules for plan writing

Plans maximize throughput through two levels of parallelism: subagents within stages (FIRST priority), and concurrent worktree stages (SECOND priority).

## Instructions

### 1. Output Location

**MANDATORY:** Write all plans to:

```text
doc/plans/PLAN-<description>.md
```

**NEVER** write to `~/.claude/plans/` or any `.claude/plans` path.

### 2. Parallelization Strategy

Maximize parallel execution at TWO levels:

```text
┌─────────────────────────────────────────────────────────────────────┐
│  PARALLELIZATION PRIORITY                                           │
│                                                                     │
│  1. SUBAGENTS FIRST  - Within a stage, use parallel subagents       │
│                        for tasks with NO file overlap               │
│                                                                     │
│  2. STAGES SECOND    - Separate stages for tasks that WILL touch    │
│                        the same files (loom merges branches)        │
└─────────────────────────────────────────────────────────────────────┘
```

| Files Overlap? | Solution                           |
| -------------- | ---------------------------------- |
| NO             | Same stage, parallel subagents     |
| YES            | Separate stages, loom merges later |

### 3. Stage Description Requirement

**EVERY stage description MUST include this line:**

```text
Use parallel subagents and skills to maximize performance.
```

This ensures Claude Code instances spawn concurrent subagents for independent tasks.

### 4. Plan Structure

Every plan MUST follow this structure:

```text
┌─────────────────────────────────────────────────────────────────────┐
│  MANDATORY PLAN STRUCTURE                                           │
│                                                                     │
│  FIRST:  knowledge-bootstrap    (unless knowledge already exists)   │
│  MIDDLE: implementation stages  (parallelized where possible)       │
│  LAST:   integration-verify     (ALWAYS - no exceptions)            │
└─────────────────────────────────────────────────────────────────────┘
```

Include a visual execution diagram:

```text
[knowledge-bootstrap] --> [stage-a, stage-b] --> [stage-c] --> [integration-verify]
```

Stages in `[a, b]` notation run concurrently.

### 5. Loom Metadata Format

Plans contain embedded YAML wrapped in HTML comments:

````markdown
<!-- loom METADATA -->

```yaml
loom:
  version: 1
  stages:
    - id: stage-id # Required: unique kebab-case identifier
      name: "Stage Name" # Required: human-readable display name
      description: | # Required: full task description for agent
        What this stage must accomplish.

        CRITICAL: Use parallel subagents and skills to maximize performance.

        Tasks:
        - Subtask 1 with requirements
        - Subtask 2 with requirements
      dependencies: [] # Required: array of stage IDs this depends on
      parallel_group: "grp" # Optional: concurrent execution grouping
      acceptance: # Required: verification commands
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files: # Optional: target file globs for scope
        - "src/**/*.rs"
      working_dir: "." # Required: "." for worktree root, or subdirectory like "loom"
      # REQUIRED: At least ONE of truths/artifacts/wiring per stage
      truths: # Observable behaviors proving feature works
        - "myapp --help"
      artifacts: # Files that must exist with real implementation
        - "src/feature/*.rs"
      wiring: # Code patterns proving integration
        - source: "src/main.rs"
          pattern: "use feature"
          description: "Feature module is imported"
```

<!-- END loom METADATA -->
````

**YAML Formatting Rules:**

| Rule                     | Correct                 | Incorrect             |
| ------------------------ | ----------------------- | --------------------- |
| Code fence               | 3 backticks             | 4 backticks           |
| Nested code blocks       | NEVER in descriptions   | Breaks YAML parser    |
| Examples in descriptions | Use plain indented text | Do NOT use ``` fences |

**Working Directory Requirement:**

The `working_dir` field is **REQUIRED** on every stage. This forces explicit choice of where acceptance criteria run:

```yaml
working_dir: "."      # Run from worktree root
working_dir: "loom"   # Run from loom/ subdirectory
```

**Why required?** Prevents acceptance failures due to forgotten directory context. Every stage must consciously declare its execution directory.

**Examples:**

```yaml
# Project with Cargo.toml at root
- id: build-check
  acceptance:
    - "cargo test"
  working_dir: "."

# Project with Cargo.toml in loom/ subdirectory
- id: build-check
  acceptance:
    - "cargo test"
  working_dir: "loom"
```

**Mixed directories?** Create separate stages instead of inline `cd`. Each stage = one working directory.

**⚠️ CRITICAL: ALL PATHS ARE RELATIVE TO working_dir**

This is a very common mistake. ALL path fields resolve relative to `working_dir`:

- `acceptance` commands
- `artifacts` file paths
- `wiring` source paths
- `truths` command paths

```yaml
# ❌ WRONG: working_dir is "loom" but paths redundantly include "loom/"
- id: implement-feature
  working_dir: "loom"
  artifacts:
    - "loom/src/feature.rs"      # WRONG: becomes loom/loom/src/feature.rs
  wiring:
    - source: "loom/src/main.rs" # WRONG: becomes loom/loom/src/main.rs
      pattern: "mod feature"

# ✅ CORRECT: Paths relative to working_dir
- id: implement-feature
  working_dir: "loom"
  artifacts:
    - "src/feature.rs"           # CORRECT: resolves to loom/src/feature.rs
  wiring:
    - source: "src/main.rs"      # CORRECT: resolves to loom/src/main.rs
      pattern: "mod feature"
```

**Rule:** If `working_dir: "loom"`, write paths as if you're already IN `loom/`.

### 6. Goal-Backward Verification (REQUIRED)

**Every stage MUST have at least ONE of: truths, artifacts, or wiring.**

These fields verify the feature actually works, not just that tests pass:

| Field | Purpose | Example |
|-------|---------|---------|
| `truths` | Observable behaviors proving feature works | `"myapp --help"`, `"curl -f localhost:8080/health"` |
| `artifacts` | Files that must exist with real implementation | `"src/auth/*.rs"`, `"tests/auth_test.rs"` |
| `wiring` | Code patterns proving integration | source + pattern + description |

**Why required?** We have had MANY instances where tests pass but the feature is never wired up or functional. These fields catch that.

```yaml
# Example: CLI command stage
truths:
  - "myapp new-command --help"  # Command is registered and callable
artifacts:
  - "src/commands/new_command.rs"  # Implementation file exists
wiring:
  - source: "src/main.rs"
    pattern: "mod new_command"
    description: "Command module is imported in main"
  - source: "src/cli.rs"
    pattern: "NewCommand"
    description: "Command is registered in CLI"
```

**Minimum requirement:** At least ONE field with at least ONE entry. More is better for critical stages.

### 7. Knowledge Bootstrap Stage (First)

Captures codebase understanding before implementation:

```yaml
- id: knowledge-bootstrap
  name: "Bootstrap Knowledge Base"
  description: |
    Explore codebase hierarchically and populate doc/loom/knowledge/:

    Use parallel subagents and skills to maximize performance.

    Exploration order:
    1. Architecture: high-level structure, component relationships, data flow
    2. Entry points: main modules, CLI commands, API endpoints
    3. Module boundaries: public interfaces, internal vs external
    4. Patterns: error handling, state management, common idioms
    5. Conventions: naming, file structure, testing patterns

    Use loom knowledge update commands to capture findings:
      loom knowledge update architecture "## Section\n\nContent..."
      loom knowledge update entry-points "## Section\n\nContent..."
      loom knowledge update patterns "## Section\n\nContent..."
      loom knowledge update conventions "## Section\n\nContent..."

    IMPORTANT: Before completing, review existing mistakes.md to avoid repeating errors.

    MEMORY RECORDING:
    - As you explore, record insights: loom memory note "observation"
    - Record decisions: loom memory decision "choice" --context "why"
    - Before completing: loom memory promote all mistakes
  dependencies: []
  acceptance:
    - "grep -q '## ' doc/loom/knowledge/architecture.md"
    - "grep -q '## ' doc/loom/knowledge/entry-points.md"
    - "grep -q '## ' doc/loom/knowledge/patterns.md"
    - "grep -q '## ' doc/loom/knowledge/conventions.md"
  files:
    - "doc/loom/knowledge/**"
  working_dir: "."  # REQUIRED: "." for worktree root
  # REQUIRED: At least one verification field
  artifacts:
    - "doc/loom/knowledge/architecture.md"
    - "doc/loom/knowledge/entry-points.md"
```

**Skip ONLY if:** `doc/loom/knowledge/` already populated or user explicitly states knowledge exists.

### 8. Integration Verify Stage (Last)

Verifies all work integrates correctly after merges AND that the feature actually works:

```text
┌─────────────────────────────────────────────────────────────────────┐
│  ⚠️ CRITICAL: TESTS PASSING ≠ FEATURE WORKING                       │
│                                                                     │
│  We have had MANY instances where:                                  │
│  - All tests pass                                                   │
│  - Code compiles                                                    │
│  - But the feature is NEVER WIRED UP or FUNCTIONAL                  │
│                                                                     │
│  integration-verify MUST include FUNCTIONAL VERIFICATION:           │
│  - Can you actually USE the feature?                                │
│  - Is it wired into the application (routes, UI, CLI)?              │
│  - Does it produce the expected user-visible behavior?              │
└─────────────────────────────────────────────────────────────────────┘
```

```yaml
- id: integration-verify
  name: "Integration Verification"
  description: |
    Final integration verification - runs AFTER all feature stages complete.

    Use parallel subagents and skills to maximize performance.

    CRITICAL: This stage must verify FUNCTIONAL INTEGRATION, not just tests passing.
    Code that compiles and passes tests but is never wired up is USELESS.

    Tasks:
    1. Run full test suite (all tests, not just affected)
    2. Run linting with warnings as errors
    3. Verify build succeeds
    4. Check for unintended regressions

    FUNCTIONAL VERIFICATION (MANDATORY):
    5. Verify the feature is actually WIRED INTO the application:
       - For CLI: Is the command registered and callable?
       - For API: Is the endpoint mounted and reachable?
       - For UI: Is the component rendered and interactive?
    6. Execute a manual smoke test of the PRIMARY USE CASE:
       - Run the actual feature end-to-end
       - Verify it produces expected output/behavior
       - Document the test steps and results
    7. Verify integration points with existing code:
       - Are callbacks/hooks connected?
       - Are events being published/subscribed?
       - Are dependencies injected correctly?

    KNOWLEDGE (MANDATORY):
    8. Review and promote session memory:
       loom memory list
       loom memory promote all mistakes
       loom memory promote decision patterns
    9. Update architecture.md if structure changed
    10. Record any lessons learned
  dependencies: ["stage-a", "stage-b", "stage-c"] # ALL feature stages
  acceptance:
    - "cargo test"
    - "cargo clippy -- -D warnings"
    - "cargo build"
    # ADD FUNCTIONAL ACCEPTANCE CRITERIA - examples:
    # - "./target/debug/myapp --help | grep 'new-command'"  # CLI wired
    # - "curl -s localhost:8080/api/new-endpoint | jq .status"  # API wired
    # - "grep -q 'NewComponent' src/app/routes.tsx"  # UI wired
  files: [] # Verification only - no file modifications
  working_dir: "."  # REQUIRED: "." for worktree root, or subdirectory like "loom"
  # REQUIRED: At least one verification field
  truths:
    - "myapp new-command --help"  # Feature is callable (adapt to YOUR feature)
  wiring:
    - source: "src/main.rs"
      pattern: "new_feature"
      description: "Feature is wired into main"
```

**Why integration-verify is mandatory:**

| Reason                  | Explanation                                        |
| ----------------------- | -------------------------------------------------- |
| Isolated worktrees      | Feature stages test locally, not globally          |
| Merge conflicts         | Individual tests pass but merged code may conflict |
| Cross-stage regressions | Stage A change may break Stage B functionality     |
| Single verification     | One authoritative pass/fail for entire plan        |
| **Wiring verification** | **Features must be connected to actually work**    |
| **Functional proof**    | **Smoke test proves the feature is usable**        |

### 9. Memory Recording in Stage Descriptions

**Every stage description should remind agents to record memory.** Memory persists insights across sessions and prevents repeated mistakes.

```text
┌─────────────────────────────────────────────────────────────────────┐
│  ⚠️  IMPLEMENTATION STAGES: Use `loom memory` ONLY                   │
│                                                                     │
│  Implementation stages must NEVER use `loom knowledge update`.      │
│  Only knowledge-bootstrap and integration-verify stages can write   │
│  to knowledge files directly.                                       │
│                                                                     │
│  Memory gets promoted to knowledge during integration-verify.       │
└─────────────────────────────────────────────────────────────────────┘
```

Include a MEMORY RECORDING block in stage descriptions:

```yaml
description: |
  [Task description here]

  MEMORY RECORDING (use memory ONLY - never knowledge):
  - Record insights: loom memory note "observation"
  - Record decisions: loom memory decision "choice" --context "why"
```

**Why this is mandatory:**

| Benefit | Explanation |
| ------- | ----------- |
| Insight persistence | Memory entries persist across sessions and context resets |
| Mistake prevention | Promoted mistakes become knowledge that future agents read |
| Decision documentation | Records WHY choices were made, not just what was done |
| Learning transfer | Memory → Knowledge transfer makes lessons permanent |

### 10. Memory vs Knowledge Rules

**CRITICAL: Different stages have different recording permissions.**

| Stage Type | `loom memory` | `loom knowledge` |
|------------|---------------|------------------|
| knowledge-bootstrap | YES | YES |
| Implementation stages | YES (ONLY) | **FORBIDDEN** |
| integration-verify | YES | YES (promote only) |

**Why this separation?**

- **Memory** is session-scoped and temporary - captures all insights during work
- **Knowledge** is permanent and shared across all stages - only proven patterns belong here
- Only after full integration (integration-verify) do we know which insights are worth keeping permanently

**The Workflow:**

1. **knowledge-bootstrap**: Directly writes to knowledge files (architecture, patterns, conventions)
2. **Implementation stages**: Record EVERYTHING to memory, NEVER touch knowledge
3. **integration-verify**: Reviews memory, promotes valuable insights to knowledge

**Implementation Stage Rule:**

During implementation stages, you MUST:

- Record insights with `loom memory note "..."`
- Record decisions with `loom memory decision "..." --context "..."`
- **NEVER** use `loom knowledge update` - this is FORBIDDEN

**Exception:** If you discover a CRITICAL MISTAKE that would block other stages, record it immediately with `loom knowledge update mistakes "..."` AND document why in your commit message.

### 11. After Writing Plan

1. Write plan to `doc/plans/PLAN-<name>.md`
2. **STOP** - Do NOT implement
3. Tell user:
   > Plan written to `doc/plans/PLAN-<name>.md`. Please review and run:
   > `loom init doc/plans/PLAN-<name>.md && loom run`
4. Wait for user feedback

**The plan file IS your deliverable.** Never proceed to implementation.

## Best Practices

1. **Subagents First**: Always maximize parallelism within stages before creating separate stages
2. **Explicit Dependencies**: Never create unnecessary sequential dependencies
3. **Clear File Scopes**: Define `files:` arrays to make overlap analysis explicit
4. **Actionable Descriptions**: Each description should be a complete task specification
5. **Testable Acceptance**: Every acceptance criterion must be a runnable command
6. **Bookend Compliance**: Always include knowledge-bootstrap first and integration-verify last
7. **Working Directory**: Every stage must declare its `working_dir` explicitly
8. **Goal-Backward Verification**: Every stage MUST have at least one of `truths`, `artifacts`, or `wiring`

## Examples

### Example 1: Parallel Stages (No File Overlap)

```yaml
# Good - stages can run concurrently
stages:
  - id: add-auth
    dependencies: ["knowledge-bootstrap"]
    files: ["src/auth/**"]
    working_dir: "."
    artifacts: ["src/auth/mod.rs"]
  - id: add-logging
    dependencies: ["knowledge-bootstrap"]
    files: ["src/logging/**"]
    working_dir: "."
    artifacts: ["src/logging/mod.rs"]
  - id: integration-verify
    dependencies: ["add-auth", "add-logging"]
    working_dir: "."
    truths: ["myapp --help"]
```

### Example 2: Sequential Stages (Same Files)

```yaml
# Both touch src/api/handler.rs - must be sequential
stages:
  - id: add-auth-to-handler
    dependencies: ["knowledge-bootstrap"]
    files: ["src/api/handler.rs"]
    working_dir: "."
    wiring:
      - source: "src/api/handler.rs"
        pattern: "auth_middleware"
        description: "Auth middleware applied to handler"
  - id: add-logging-to-handler
    dependencies: ["add-auth-to-handler"] # Sequential
    files: ["src/api/handler.rs"]
    working_dir: "."
    wiring:
      - source: "src/api/handler.rs"
        pattern: "log_request"
        description: "Request logging added to handler"
  - id: integration-verify
    dependencies: ["add-logging-to-handler"]
    working_dir: "."
    truths: ["curl -f localhost:8080/api/health"]
```

### Example 3: Complete Plan Template

````markdown
# Plan: [Title]

## Overview

[2-3 sentence description]

## Execution Diagram

```
[knowledge-bootstrap] --> [stage-a, stage-b] --> [integration-verify]
```

<!-- loom METADATA -->

```yaml
loom:
  version: 1
  stages:
    - id: knowledge-bootstrap
      name: "Bootstrap Knowledge Base"
      description: |
        Explore codebase and populate doc/loom/knowledge/.

        Use parallel subagents and skills to maximize performance.

        Tasks:
        - Identify entry points and main modules
        - Document patterns and conventions
      dependencies: []
      acceptance:
        - "grep -q '## ' doc/loom/knowledge/entry-points.md"
      files:
        - "doc/loom/knowledge/**"
      working_dir: "."
      artifacts:
        - "doc/loom/knowledge/architecture.md"
        - "doc/loom/knowledge/entry-points.md"

    - id: stage-a
      name: "Feature A"
      description: |
        Implement feature A.

        Use parallel subagents and skills to maximize performance.

        Tasks:
        - Task 1
        - Task 2
      dependencies: ["knowledge-bootstrap"]
      acceptance:
        - "cargo test"
      files:
        - "src/feature_a/**"
      working_dir: "."
      artifacts:
        - "src/feature_a/mod.rs"

    - id: stage-b
      name: "Feature B"
      description: |
        Implement feature B.

        Use parallel subagents and skills to maximize performance.

        Tasks:
        - Task 1
        - Task 2
      dependencies: ["knowledge-bootstrap"]
      acceptance:
        - "cargo test"
      files:
        - "src/feature_b/**"
      working_dir: "."
      artifacts:
        - "src/feature_b/mod.rs"

    - id: integration-verify
      name: "Integration Verification"
      description: |
        Final verification after all stages complete.

        Use parallel subagents and skills to maximize performance.

        CRITICAL: Verify FUNCTIONAL INTEGRATION, not just tests passing.

        Build/Test Tasks:
        - Full test suite
        - Linting
        - Build verification

        FUNCTIONAL VERIFICATION (MANDATORY):
        - Verify features are WIRED into the application
        - Execute smoke test of primary use case
        - Confirm user-visible behavior works end-to-end
      dependencies: ["stage-a", "stage-b"]
      acceptance:
        - "cargo test"
        - "cargo clippy -- -D warnings"
        - "cargo build"
        # ADD: Functional acceptance criteria for YOUR feature
      files: []
      working_dir: "."
      truths:
        - "myapp --help"  # Adapt to YOUR feature
      wiring:
        - source: "src/main.rs"
          pattern: "feature_a"
          description: "Feature A is wired into main"
```

<!-- END loom METADATA -->
````
