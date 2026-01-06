# Claude Code Agent & Skills Library

A comprehensive collection of hierarchical AI agents and reusable skills for Claude Code CLI, designed to optimize performance across software engineering, security, machine learning, infrastructure, product design, analytics, QA, data engineering, and technical writing domains.

## Overview

This project provides:

- **19 Specialized Agents** - Senior (opus) and Standard (sonnet) pairs across 8 domains, plus Security Engineer, Tech Lead, and Doc Editor
- **48 Reusable Skills** - Modular capabilities that agents can leverage

## Why Agents + Skills?

Claude Code's power comes from combining **agents** (specialized subagents with focused expertise) and **skills** (reusable knowledge modules). This architecture provides:

### Context Efficiency

Each subagent runs in its own context window. Instead of one massive conversation that hits context limits, work is distributed across multiple focused agents. This means:

- **Larger projects**: Break down 50-file refactors into parallel subagent tasks
- **Preserved context**: The main conversation stays clean while subagents handle details
- **Better results**: Each agent focuses on its specialty without context pollution

### Versatility Through Composition

Skills are loaded dynamically based on the task. A `software-engineer` agent working on a Python API can automatically load `python`, `api-design`, and `testing` skills—getting specialized knowledge without bloating every conversation.

### Parallel Execution

Subagents can run in parallel (up to 10 concurrent). A single prompt like "refactor authentication across all services" can spawn multiple agents working simultaneously on different files, dramatically reducing total time.

> **Learn more**: [Subagents Documentation](https://code.claude.com/docs/en/sub-agents) · [Agent Skills Blog Post](https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills)

## Installation

### Prerequisites

- [Claude Code CLI](https://code.claude.com/docs/en/overview) installed and configured

### Quick Install

**One-liner (recommended):**

```bash
curl -fsSL https://raw.githubusercontent.com/cosmix/cluade-flux/main/install.sh | bash
```

**Or clone and install:**

```bash
git clone https://github.com/cosmix/cluade-flux.git
cd cluade-flux
bash install.sh
```

The installer will:

- Copy agents and skills to `~/.claude/`
- Append orchestration rules to `~/.claude/CLAUDE.md`
- Back up any existing files (as `*.bak.<timestamp>`)

### Manual Install

If you prefer to install manually or to a specific project:

```bash
# User-level (available in all projects)
cp -r agents ~/.claude/
cp -r skills ~/.claude/
cat CLAUDE.template.md >> ~/.claude/CLAUDE.md

# Project-level (team-shared via git)
cp -r agents /path/to/your/project/.claude/
cp -r skills /path/to/your/project/.claude/
cat CLAUDE.template.md >> /path/to/your/project/CLAUDE.md
```

### What Gets Installed

The [`CLAUDE.template.md`](CLAUDE.template.md) configuration includes:

- **Agent orchestration** - When to use senior (opus) vs standard (sonnet) agents
- **Parallel/sequential execution** - Guidelines for spawning agents efficiently
- **Context passing** - What information to provide subagents
- **Development standards** - Implementation, planning, documentation, code quality
- **Dependency management** - Always use package managers, never edit manifests manually
- **Progress tracking** - How to record and clean up task progress

### Verify Installation

Start a Claude Code session and check that agents and skills are loaded:

```bash
claude

# Inside Claude Code, ask:
> What agents are available?
> What skills do you have access to?
```

> **Official Documentation**:
>
> - [Subagents Guide](https://code.claude.com/docs/en/sub-agents)
> - [Skills Introduction](https://claude.com/blog/skills)
> - [Claude Code Best Practices](https://www.anthropic.com/engineering/claude-code-best-practices)

## How the CLAUDE.md Configuration Works

The CLAUDE.md instructions you add to `~/.claude/CLAUDE.md` become part of Claude's system context. They guide Claude to:

1. **Recognize delegation opportunities** - When you ask for a complex task, Claude checks if specialist agents can handle parts of it

2. **Choose the right agent tier** - Senior agents (opus) for architecture/debugging, standard agents (sonnet) for implementation

3. **Parallelize independent work** - Claude spawns multiple subagents simultaneously when tasks don't depend on each other

4. **Manage context efficiently** - Instead of trying to do everything in one context window, Claude distributes work across focused subagents

### Example: Multi-File Refactoring

Without orchestration rules, Claude might try to refactor 20 files sequentially in one context, eventually hitting limits.

With orchestration rules, Claude will:

```text
1. Use tech-lead to analyze the refactoring scope
2. Spawn senior-software-engineer to design the approach
3. Spawn multiple software-engineer agents IN PARALLEL to refactor different files
4. Each agent loads relevant skills (python, refactoring, testing)
5. Results merge back without exhausting main context
```

## Project Specifications (Workflows)

Your project-specific workflows and specifications should be documented in your project's `CLAUDE.md` file. This ensures Claude understands your team's processes and can follow them automatically.

### Adding Workflows to CLAUDE.md

You can include workflows directly in your project's CLAUDE.md:

```markdown
# Project Workflows

## Feature Development

1. Create feature branch from main
2. Write tests first (TDD)
3. Implement feature
4. Run full test suite
5. Create PR with template
```

### Pointing to External Sources

Alternatively, reference external documentation:

```markdown
# Project Workflows

- Feature specs: [Notion workspace](https://notion.so/team/features)
- Sprint planning: [Linear project board](https://linear.app/team/project)
- API contracts: [Confluence page](https://confluence.company.com/api-specs)
```

**Note:** External sources may require MCP (Model Context Protocol) servers to be configured for Claude to access them directly. Check the [MCP documentation](https://modelcontextprotocol.io) for integration details.

### Best Practices

- Keep workflows concise and actionable
- Update CLAUDE.md when processes change
- Include links to detailed documentation for complex workflows
- Document team-specific conventions and standards

## Flux CLI: Enabling Agent Orchestration

Flux is a command-line tool for managing persistent work state across AI agent sessions. It implements the **Signal Principle**: _"If you have a signal, answer it"_ — enabling agents to resume work automatically without manual context restoration.

### What Flux Does

Flux provides structured work state management:

- **Runners**: Individual agent instances (e.g., `se-001`, `tl-001`) with specific roles
- **Tracks**: Grouped units of work (features, bugs, refactors)
- **Signals**: Pending work items that need attention
- **State Persistence**: All state stored in `.work/` directory as markdown files (git-friendly)

When you start Claude, it reads the `.work/` state and automatically picks up any signals assigned to it, enabling true self-propelling workflows.

### Installing Flux

Flux is installed automatically by `install.sh`. The binary is placed in `~/.local/bin/` (or `/usr/local/bin/` if `~/.local/bin/` doesn't exist).

**Manual Installation:**

1. Download the latest binary for your platform from [GitHub Releases](https://github.com/cosmix/cluade-flux/releases)
2. Make it executable: `chmod +x flux`
3. Move to your PATH: `mv flux ~/.local/bin/` or `mv flux /usr/local/bin/`

### Updating

To update flux and all configuration files to the latest version:

```bash
flux self-update
```

This will:

- Check GitHub for the latest release
- Download and replace the flux binary
- Update agents and skills in `~/.claude/`
- Update `~/.claude/CLAUDE.md` with the latest template

### Commands Reference

#### Initialization and Health

```bash
flux init                          # Initialize .work/ directory in current project
flux status                        # Show dashboard with runners, tracks, signals
flux validate                      # Check state integrity
flux doctor                        # Diagnose configuration issues
```

#### Track Management

```bash
flux track new "Feature Name"      # Create new work track
flux track list                    # List all tracks
flux track show <id>               # View track details
flux track close <id>              # Close completed track
```

#### Runner Management

```bash
flux runner create <name> -t <type>    # Create runner (types: se-001, sse-001, tl-001, etc.)
flux runner list                       # List all runners
flux runner assign <runner> <track>    # Assign runner to track
flux runner release <runner>           # Release runner from current track
```

**Runner Types:**

- `software-engineer` (se-001, se-002, ...)
- `senior-software-engineer` (sse-001, sse-002, ...)
- `tech-lead` (tl-001)
- `security-engineer` (sec-001)
- And more matching your agent hierarchy

#### Signal Management

```bash
flux signal set <runner> <type> <message>  # Send signal to runner
flux signal show                           # View all pending signals
flux signal clear <id>                     # Clear/complete signal
```

**Signal Types:**

- `start`: Begin new work
- `review`: Code review request
- `debug`: Investigation needed
- `test`: Testing required
- `document`: Documentation task

### Workflow Example

```bash
# Initialize flux in your project
cd /path/to/project
flux init

# Create a new feature track
flux track new "User Authentication"
# Output: Created track: user-authentication (t-001)

# Create a runner and assign it
flux runner create auth-impl -t software-engineer
# Output: Created runner: se-001

flux runner assign se-001 user-authentication
flux signal set se-001 start "Implement JWT-based authentication with refresh tokens"

# Check status
flux status
# Output shows:
# - Runner se-001 assigned to track t-001
# - Pending signal: start for se-001

# In another terminal, start Claude
claude
# Claude reads .work/ state, sees the signal, and begins work automatically
```

### Integration with Agent Workflows

Flux works seamlessly with the agent hierarchy:

1. **Tech Lead** creates tracks and assigns runners
2. **Senior Engineers** design architecture and create implementation signals
3. **Standard Engineers** execute implementation work
4. **Security Engineer** reviews security-sensitive changes
5. **QA Engineers** validate functionality

Each agent checks for signals on startup and can create new signals for other agents, enabling true autonomous orchestration.

### State Persistence

All Flux state lives in `.work/` as markdown files:

```text
.work/
├── runners/
│   ├── se-001.md          # Runner state
│   └── tl-001.md
├── tracks/
│   ├── t-001.md           # Track details
│   └── t-002.md
└── signals/
    └── s-001.md           # Pending signals
```

This git-friendly format means:

- State survives across sessions
- Team can collaborate on work state
- Full audit trail in version control
- Easy manual editing if needed

## Agent Hierarchy

Each domain has two agents with distinct responsibilities:

| Domain               | Senior (opus)                      | Standard (sonnet)         |
| -------------------- | ---------------------------------- | ------------------------- |
| Software Engineering | `senior-software-engineer`         | `software-engineer`       |
| Security             | `security-engineer` (single agent) | —                         |
| Machine Learning     | `senior-ml-engineer`               | `ml-engineer`             |
| Infrastructure       | `senior-infrastructure-engineer`   | `infrastructure-engineer` |
| Product Design       | `senior-product-designer`          | `product-designer`        |
| Analytics            | `senior-data-analyst`              | `data-analyst`            |
| Quality Assurance    | `senior-qa-engineer`               | `qa-engineer`             |
| Data Engineering     | `senior-data-engineer`             | `data-engineer`           |
| Technical Writing    | `senior-technical-writer`          | `technical-writer`        |

### Special Agents

| Agent        | Model | Purpose                                                            |
| ------------ | ----- | ------------------------------------------------------------------ |
| `tech-lead`  | opus  | Cross-functional coordination, project planning, work distribution |
| `doc-editor` | haiku | Markdown linting, formatting fixes, documentation consistency      |

### When to Use Senior Agents (opus)

Use senior agents for higher-level thinking and complex work:

- **Planning** - System design, project architecture, implementation strategies
- **Architecture** - Component design, API contracts, data modeling decisions
- **Difficult Algorithms** - Complex logic, optimization problems, novel solutions
- **Design Patterns** - Selecting and applying appropriate patterns
- **Debugging** - Root cause analysis of complex issues
- **Code Review** - Evaluating design decisions and code quality
- **Strategic Decisions** - Technology selection, trade-off analysis

### When to Use Standard Agents (sonnet)

Use standard agents for implementation and routine work:

- **Boilerplate Code** - Standard implementations when detailed specs for them exist, CRUD operations
- **Well-Defined Components** - Fleshing out specs that are already designed
- **Routine Tasks** - Following established patterns and conventions
- **Standard Configurations** - Writing configs, manifests, pipelines
- **Data Processing** - ETL, preprocessing, standard transformations
- **Documentation** - Writing docs for implemented features

## Skills Library

Skills provide modular capabilities that agents can invoke. They are loaded dynamically based on the task context.

### Language Expertise

| Skill        | Description                                         |
| ------------ | --------------------------------------------------- |
| `python`     | Pythonic idioms, type hints, async patterns, pytest |
| `golang`     | Go idioms, error handling, concurrency, modules     |
| `rust`       | Ownership, lifetimes, error handling, cargo, traits |
| `typescript` | Type system, generics, utility types, strict mode   |

### Code Quality

| Skill           | Description                                                    |
| --------------- | -------------------------------------------------------------- |
| `code-review`   | Comprehensive code reviews for correctness and maintainability |
| `refactoring`   | Restructure code without changing behavior                     |
| `testing`       | Create unit, integration, and e2e test suites                  |
| `documentation` | Generate technical docs, API references, READMEs               |

### Development

| Skill             | Description                                          |
| ----------------- | ---------------------------------------------------- |
| `api-design`      | Design RESTful APIs, GraphQL schemas, RPC interfaces |
| `database-design` | Design schemas, relationships, indexes, migrations   |
| `git-workflow`    | Branching strategies, commits, conflict resolution   |
| `debugging`       | Systematic bug diagnosis and resolution              |

### Documentation

| Skill               | Description                                     |
| ------------------- | ----------------------------------------------- |
| `technical-writing` | Clear prose, audience-aware docs, structure     |
| `diagramming`       | Mermaid diagrams, architecture, sequences, ERDs |
| `api-documentation` | OpenAPI specs, endpoint docs, SDK documentation |
| `md-tables`         | Markdown table alignment and spacing fixes      |

### QA & Testing

| Skill                 | Description                                      |
| --------------------- | ------------------------------------------------ |
| `test-strategy`       | Test pyramid, coverage goals, what/when to test  |
| `e2e-testing`         | Playwright/Cypress patterns, fixtures, selectors |
| `performance-testing` | Load testing, benchmarking, profiling            |

### Infrastructure

| Skill        | Description                                 |
| ------------ | ------------------------------------------- |
| `docker`     | Dockerfiles and docker-compose optimization |
| `kubernetes` | K8s deployments, services, configurations   |
| `terraform`  | Infrastructure as Code for cloud resources  |
| `ci-cd`      | CI/CD pipeline design and implementation    |

### Security

| Skill             | Description                                |
| ----------------- | ------------------------------------------ |
| `security-audit`  | Comprehensive vulnerability assessment     |
| `security-scan`   | Quick routine checks (secrets, deps, SAST) |
| `threat-model`    | STRIDE/DREAD analysis, secure architecture |
| `dependency-scan` | CVE scanning and license compliance        |
| `auth`            | OAuth2, JWT, RBAC/ABAC, session management |

### Reliability & Operations

| Skill                   | Description                                    |
| ----------------------- | ---------------------------------------------- |
| `error-handling`        | Error types, recovery strategies, propagation  |
| `logging-observability` | Structured logging, tracing, metrics, alerts   |
| `concurrency`           | Async patterns, parallelism, race conditions   |
| `caching`               | Cache strategies, invalidation, Redis patterns |
| `code-migration`        | Version upgrades, framework migrations         |
| `rate-limiting`         | Throttling, backpressure, API quotas           |

### Architecture Patterns

| Skill             | Description                                    |
| ----------------- | ---------------------------------------------- |
| `event-driven`    | Message queues, pub/sub, event sourcing, CQRS  |
| `feature-flags`   | Rollouts, A/B testing, kill switches           |
| `background-jobs` | Job queues, schedulers, workers, idempotency   |
| `webhooks`        | Design, verification, retry logic, idempotency |
| `serialization`   | JSON/protobuf/msgpack, schema evolution        |

### Data

| Skill                | Description                                |
| -------------------- | ------------------------------------------ |
| `sql-optimization`   | Query analysis and performance tuning      |
| `data-visualization` | Charts, dashboards, visual analytics       |
| `data-validation`    | Schema validation, sanitization, contracts |
| `search`             | Elasticsearch, full-text search, indexing  |

### AI/ML

| Skill                | Description                               |
| -------------------- | ----------------------------------------- |
| `prompt-engineering` | LLM prompt design and optimization        |
| `model-evaluation`   | ML model performance and fairness testing |

### Frontend

| Skill           | Description                                   |
| --------------- | --------------------------------------------- |
| `accessibility` | WCAG compliance, a11y testing, screen readers |
| `i18n`          | Internationalization, translations, RTL       |
| `react`         | React patterns, hooks, state management       |

## Usage

### Using Agents

Agents are automatically invoked by Claude Code when tasks match their descriptions. You can also explicitly request them:

```text
Use the senior-software-engineer agent to design the architecture
```

```text
Have the ml-engineer preprocess this dataset
```

### Using Skills

Skills are model-invoked based on context. Claude will automatically use relevant skills when appropriate:

```text
Use the python skill for this implementation
```

```text
Apply the e2e-testing skill to write Playwright tests
```

## Customization

### Adding New Agents

Create a new `.md` file in `agents/`:

```markdown
---
name: my-agent
description: What this agent does. Use PROACTIVELY when relevant.
tools: Read, Edit, Write, Glob, Grep, Bash
model: sonnet
---

Your agent's system prompt here.
```

### Adding New Skills

Create a new directory in `skills/` with a `SKILL.md`:

```markdown
---
name: my-skill
description: What this skill does and when to use it.
allowed-tools: Read, Grep, Glob
---

# My Skill

## Instructions

Step-by-step guidance.

## Best Practices

Key principles.

## Examples

Concrete examples.
```

## Further Reading

- [Claude Code Documentation](https://code.claude.com/docs/en/overview)
- [Subagents Deep Dive](https://code.claude.com/docs/en/sub-agents)
- [Agent Skills Introduction](https://claude.com/blog/skills)
- [Building Agents with Claude Agent SDK](https://www.anthropic.com/engineering/building-agents-with-the-claude-agent-sdk)
- [Claude Code Best Practices](https://www.anthropic.com/engineering/claude-code-best-practices)

## License

MIT
