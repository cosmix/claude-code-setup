---
name: git-workflow
description: Manages Git operations including branching strategies, commit conventions, merge workflows, and conflict resolution. Trigger keywords: git, branch, commit, merge, rebase, PR, pull request, conflict, version control.
allowed-tools: Read, Grep, Glob, Bash
---

# Git Workflow

## Overview

This skill provides guidance on Git best practices, branching strategies, commit conventions, and collaborative workflows. It helps maintain a clean and navigable version control history.

## Instructions

### 1. Branch Management

- Follow consistent naming conventions
- Create branches from appropriate base
- Keep branches focused and short-lived
- Delete merged branches promptly

### 2. Commit Practices

- Write meaningful commit messages
- Make atomic commits (one logical change)
- Use conventional commit format
- Sign commits when required

### 3. Merge Strategies

- Choose appropriate merge strategy
- Review changes before merging
- Resolve conflicts carefully
- Maintain clean history

### 4. Collaboration

- Keep branches up to date
- Use pull requests for review
- Squash commits when appropriate
- Protect important branches

## Best Practices

1. **Atomic Commits**: Each commit should represent one logical change
2. **Meaningful Messages**: Describe what and why, not how
3. **Branch Often**: Use feature branches for all changes
4. **Pull Before Push**: Stay synchronized with remote
5. **Review Before Merge**: All changes should be reviewed
6. **Protect Main**: Never force push to main/master
7. **Clean History**: Squash WIP commits before merging

## Examples

### Example 1: Conventional Commit Messages

```bash
# Format: <type>(<scope>): <description>
# Types: feat, fix, docs, style, refactor, test, chore

# Feature addition
git commit -m "feat(auth): add OAuth2 login with Google provider"

# Bug fix with issue reference
git commit -m "fix(cart): resolve race condition in quantity update

When rapidly clicking add/remove, the cart count could become negative
due to unsynchronized state updates.

Fixes #234"

# Breaking change
git commit -m "feat(api)!: change user endpoint response format

BREAKING CHANGE: The /users endpoint now returns paginated results
instead of an array. Clients must update to handle the new format.

Migration guide: https://docs.example.com/migration/v2"

# Documentation
git commit -m "docs(readme): add installation instructions for Windows"

# Refactoring
git commit -m "refactor(db): extract query builder into separate module"
```

### Example 2: Branch Naming Conventions

```bash
# Feature branches
git checkout -b feature/user-authentication
git checkout -b feature/JIRA-123-shopping-cart

# Bug fix branches
git checkout -b fix/login-redirect-loop
git checkout -b fix/JIRA-456-null-pointer

# Hotfix branches (production issues)
git checkout -b hotfix/security-patch-xss

# Release branches
git checkout -b release/v2.1.0

# Experiment branches
git checkout -b experiment/new-caching-strategy
```

### Example 3: Git Workflow Commands

```bash
# Start new feature
git checkout main
git pull origin main
git checkout -b feature/new-feature

# Regular development cycle
git add -A
git commit -m "feat: implement feature part 1"
git push -u origin feature/new-feature

# Keep branch updated with main
git fetch origin
git rebase origin/main
# Or merge if preferred
git merge origin/main

# Interactive rebase to clean up commits before PR
git rebase -i origin/main
# In editor: squash, reword, or reorder commits

# After PR approval, merge and cleanup
git checkout main
git pull origin main
git branch -d feature/new-feature
git push origin --delete feature/new-feature

# Handling merge conflicts
git merge feature-branch
# If conflicts occur:
git status  # See conflicted files
# Edit files to resolve conflicts
git add <resolved-files>
git merge --continue

# Undo last commit (keep changes)
git reset --soft HEAD~1

# Undo last commit (discard changes)
git reset --hard HEAD~1

# Cherry-pick specific commit
git cherry-pick abc123

# Create annotated tag for release
git tag -a v1.0.0 -m "Release version 1.0.0"
git push origin v1.0.0
```

### Example 4: Git Aliases for Productivity

```bash
# Add to ~/.gitconfig
[alias]
    co = checkout
    br = branch
    ci = commit
    st = status
    lg = log --oneline --graph --decorate
    unstage = reset HEAD --
    last = log -1 HEAD
    amend = commit --amend --no-edit
    wip = commit -am "WIP"
    undo = reset --soft HEAD~1
    branches = branch -a
    tags = tag -l
    stashes = stash list

    # Show branches sorted by last commit date
    recent = for-each-ref --sort=-committerdate refs/heads/ --format='%(refname:short) %(committerdate:relative)'

    # Delete all merged branches
    cleanup = "!git branch --merged | grep -v '\\*\\|main\\|master' | xargs -n 1 git branch -d"
```
