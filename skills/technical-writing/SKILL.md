---
name: technical-writing
description: Professional technical documentation writing for software projects. Use for README files, user guides, migration guides, changelogs, and general documentation. Triggers: documentation, docs, readme, guide, tutorial, changelog, migration guide, write docs, document code.
---

# Technical Writing

## Overview

Technical writing skill for creating clear, comprehensive documentation for software projects. This skill covers the full documentation lifecycle from understanding code to producing polished, audience-appropriate documentation.

## Instructions

### 1. Understand Before Documenting

- **Read the code first**: Never document without understanding the implementation
- **Identify the purpose**: What problem does this code solve?
- **Trace the flow**: Understand how components interact
- **Note edge cases**: Document limitations and constraints
- **Check existing docs**: Review current documentation for style and gaps

### 2. Know Your Audience

- **Developers**: Focus on API details, code examples, technical depth
- **End users**: Focus on tasks, outcomes, minimal technical jargon
- **Operators/DevOps**: Focus on deployment, configuration, monitoring
- **New team members**: Focus on onboarding, architecture overview

### 3. Structure Documents Effectively

Standard documentation structure:

```
1. Overview/Introduction
   - What is this?
   - Why does it exist?
   - Who is it for?

2. Getting Started
   - Prerequisites
   - Installation
   - Quick start example

3. Core Concepts
   - Key terminology
   - Architecture overview
   - Mental models

4. Usage Guide
   - Common tasks
   - Configuration options
   - Best practices

5. API Reference
   - Methods/endpoints
   - Parameters
   - Return values
   - Examples

6. Troubleshooting
   - Common issues
   - Error messages
   - FAQ

7. Contributing (if applicable)
   - Development setup
   - Code style
   - PR process
```

### 4. Writing Process

1. **Outline first**: Create structure before prose
2. **Write draft**: Get content down without perfecting
3. **Add examples**: Include working code snippets
4. **Review for clarity**: Simplify complex sentences
5. **Check consistency**: Verify terminology and style
6. **Test examples**: Ensure code samples work

## Best Practices

### Prose Style

- Use active voice: "The function returns" not "The value is returned by"
- Be concise: Remove unnecessary words
- Be specific: "Returns a string" not "Returns the result"
- Use present tense for descriptions
- Use imperative mood for instructions: "Run the command" not "You should run"

### README Best Practices

```markdown
# Project Name

Brief one-line description.

## Features

- Key feature 1
- Key feature 2

## Installation

\`\`\`bash
npm install package-name
\`\`\`

## Quick Start

\`\`\`javascript
import { feature } from 'package-name';

const result = feature.doSomething();
\`\`\`

## Documentation

Link to full docs.

## Contributing

Link to contributing guide.

## License

MIT
```

### Changelog Best Practices

Follow [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
# Changelog

## [Unreleased]

## [1.2.0] - 2024-01-15

### Added

- New feature X for doing Y

### Changed

- Updated dependency Z to version 2.0

### Fixed

- Bug where A caused B

### Removed

- Deprecated method `oldMethod()`
```

### Migration Guide Best Practices

```markdown
# Migration Guide: v1.x to v2.0

## Overview

Brief description of why migration is needed.

## Breaking Changes

### Change 1: New Authentication

**Before (v1.x):**
\`\`\`javascript
client.auth(apiKey);
\`\`\`

**After (v2.0):**
\`\`\`javascript
client.authenticate({ key: apiKey, type: 'bearer' });
\`\`\`

## Step-by-Step Migration

1. Update package to v2.0
2. Replace auth calls (see above)
3. Update configuration file format
4. Test your integration

## Deprecation Timeline

- v2.0: Old methods deprecated with warnings
- v3.0: Old methods removed
```

## Examples

### Example: Function Documentation

```javascript
/**
 * Calculates the total price including tax and discounts.
 *
 * @param {number} basePrice - The original price before adjustments
 * @param {Object} options - Calculation options
 * @param {number} [options.taxRate=0.1] - Tax rate as decimal (0.1 = 10%)
 * @param {number} [options.discount=0] - Discount amount in currency units
 * @returns {number} Final price rounded to 2 decimal places
 * @throws {Error} If basePrice is negative
 *
 * @example
 * // Basic usage
 * calculateTotal(100);
 * // Returns: 110.00
 *
 * @example
 * // With discount
 * calculateTotal(100, { discount: 20 });
 * // Returns: 88.00
 */
function calculateTotal(basePrice, options = {}) {
  // implementation
}
```

### Example: CLI Documentation

```markdown
## Usage

\`\`\`
myapp <command> [options]
\`\`\`

### Commands

| Command | Description              |
| ------- | ------------------------ |
| `init`  | Initialize a new project |
| `build` | Build the project        |
| `serve` | Start development server |

### Options

| Option      | Alias | Description           | Default         |
| ----------- | ----- | --------------------- | --------------- |
| `--config`  | `-c`  | Path to config file   | `./config.json` |
| `--verbose` | `-v`  | Enable verbose output | `false`         |
| `--port`    | `-p`  | Server port           | `3000`          |

### Examples

Initialize a new project:
\`\`\`bash
myapp init my-project
\`\`\`

Build with custom config:
\`\`\`bash
myapp build -c ./custom-config.json
\`\`\`
```
