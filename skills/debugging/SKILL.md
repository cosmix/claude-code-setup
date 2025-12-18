---
name: debugging
description: Systematically diagnoses and resolves software bugs using various debugging techniques and tools. Trigger keywords: debug, bug, error, exception, crash, issue, troubleshoot, fix, stack trace.
allowed-tools: Read, Grep, Glob, Bash, Edit
---

# Debugging

## Overview

This skill provides systematic approaches to finding and fixing bugs. It covers debugging strategies, tool usage, and techniques for various types of issues across different environments.

## Instructions

### 1. Understand the Problem

- Reproduce the issue consistently
- Gather error messages and stack traces
- Identify when the bug was introduced
- Determine the expected vs actual behavior

### 2. Isolate the Issue

- Create minimal reproduction case
- Use binary search to narrow down cause
- Check recent changes (git bisect)
- Verify environment and dependencies

### 3. Diagnose Root Cause

- Add strategic logging
- Use debugger to step through code
- Analyze stack traces
- Check for common patterns

### 4. Fix and Verify

- Implement targeted fix
- Add regression test
- Verify fix doesn't introduce new issues
- Document the root cause

## Best Practices

1. **Reproduce First**: Never fix what you can't reproduce
2. **Read Error Messages**: They often contain the answer
3. **Check Recent Changes**: Most bugs are recently introduced
4. **Question Assumptions**: Verify what you think you know
5. **Isolate Variables**: Change one thing at a time
6. **Use Source Control**: Git bisect is powerful
7. **Write Tests**: Prove the bug exists, then prove it's fixed

## Examples

### Example 1: Systematic Debugging Process

```python
# Step 1: Understand the error
"""
Error: TypeError: Cannot read property 'name' of undefined
at processUser (src/users.py:45)
at handleRequest (src/server.py:123)
"""

# Step 2: Add diagnostic logging
def process_user(user_id: str) -> dict:
    logger.debug(f"Processing user_id: {user_id}")

    user = get_user(user_id)
    logger.debug(f"Retrieved user: {user}")  # <-- User is None!

    # Bug: No null check before accessing properties
    return {"name": user.name}  # Crashes here

# Step 3: Fix with proper null handling
def process_user(user_id: str) -> dict:
    logger.debug(f"Processing user_id: {user_id}")

    user = get_user(user_id)
    if user is None:
        logger.warning(f"User not found: {user_id}")
        raise UserNotFoundError(f"User {user_id} not found")

    return {"name": user.name}

# Step 4: Add regression test
def test_process_user_not_found():
    with pytest.raises(UserNotFoundError):
        process_user("nonexistent-id")
```

### Example 2: Git Bisect for Finding Bug Introduction

```bash
# Start bisect session
git bisect start

# Mark current commit as bad (has the bug)
git bisect bad

# Mark known good commit (before bug existed)
git bisect good v1.2.0

# Git checks out middle commit, test it
# Run your test
npm test

# Mark result
git bisect good  # or git bisect bad

# Repeat until git identifies the first bad commit
# Git will output: "abc123 is the first bad commit"

# View the problematic commit
git show abc123

# End bisect session
git bisect reset
```

### Example 3: Common Bug Patterns

```python
# Pattern 1: Off-by-one errors
# Bug: Missing last element
for i in range(len(items) - 1):  # Wrong!
    process(items[i])
# Fix:
for i in range(len(items)):
    process(items[i])

# Pattern 2: Race conditions
# Bug: Check-then-act without synchronization
if not file.exists():
    file.create()  # Another thread might create between check and create
# Fix: Use atomic operations
file.create_if_not_exists()

# Pattern 3: Floating point comparison
# Bug: Direct equality comparison
if 0.1 + 0.2 == 0.3:  # This is False!
    do_something()
# Fix: Use approximate comparison
if abs((0.1 + 0.2) - 0.3) < 1e-9:
    do_something()

# Pattern 4: Mutable default arguments
# Bug: Shared mutable default
def add_item(item, items=[]):  # Same list instance reused!
    items.append(item)
    return items
# Fix: Use None default
def add_item(item, items=None):
    if items is None:
        items = []
    items.append(item)
    return items

# Pattern 5: Silent failures
# Bug: Swallowing exceptions
try:
    risky_operation()
except Exception:
    pass  # Bug hidden!
# Fix: Handle or re-raise appropriately
try:
    risky_operation()
except SpecificException as e:
    logger.error(f"Operation failed: {e}")
    raise
```

### Example 4: Debugging Tools Usage

```bash
# Python debugging
python -m pdb script.py  # Interactive debugger
python -m trace --trace script.py  # Trace execution

# Node.js debugging
node --inspect script.js  # Chrome DevTools
node --inspect-brk script.js  # Break on first line

# Memory profiling (Python)
python -m memory_profiler script.py

# CPU profiling (Python)
python -m cProfile -o output.prof script.py
python -m pstats output.prof

# Strace for system calls (Linux)
strace -f -e trace=file python script.py

# Network debugging
tcpdump -i any port 8080
curl -v http://localhost:8080/api/health
```
