# Flux - Self-Propelling Agent Orchestration CLI

A command-line interface for managing multi-agent AI workflows with context tracking and handoff capabilities.

## Overview

Flux provides a structured workspace for coordinating multiple AI agents (runners) working on different conversation threads (tracks) with inter-agent communication via signals.

## Installation

```bash
cargo install --path .
```

## Usage

### Initialize Workspace

```bash
flux init
```

Creates a `.work/` directory with the following structure:

- `runners/` - AI agent configurations
- `tracks/` - Conversation thread metadata
- `signals/` - Inter-agent messages
- `handoffs/` - Context handoff records
- `archive/` - Archived entities

### Track Management

```bash
# Create a new track
flux track new "feature-implementation" --description "Implement new feature"

# List all tracks
flux track list

# Show track details
flux track show <track-id>

# Close a track
flux track close <track-id> --reason "Completed"
```

### Runner Management

```bash
# Create a runner
flux runner create "sonnet-1" --runner-type sonnet

# List runners
flux runner list

# Assign runner to track
flux runner assign <runner-id> <track-id>

# Release runner
flux runner release <runner-id>

# Archive runner
flux runner archive <runner-id>
```

### Signal Management

```bash
# Send signal
flux signal set <runner-id> <signal-type> "message" --priority 3

# View signals
flux signal show [runner-id]

# Clear signal
flux signal clear <signal-id>
```

### Monitoring

```bash
# View dashboard
flux status

# Validate workspace
flux validate

# Run diagnostics
flux doctor
```

## Architecture

### Models

- **Runner**: AI agent with context tracking and lifecycle management
- **Track**: Conversation thread with hierarchy support
- **Signal**: Inter-agent communication message
- **Handoff**: Context transfer between runners

### Modules

- `commands/`: CLI command implementations
- `models/`: Core data structures
- `parser/`: Markdown and data parsing utilities
- `fs/`: Filesystem and workspace management

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy

# Format code
cargo fmt
```

## License

MIT
