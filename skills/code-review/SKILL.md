---
name: code-review
description: Performs comprehensive code reviews focusing on correctness, maintainability, performance, and best practices. Trigger keywords: review, code review, PR review, pull request, check code, audit code.
allowed-tools: Read, Grep, Glob, Bash
---

# Code Review

## Overview

This skill provides thorough code review capabilities, analyzing code for bugs, design issues, performance problems, and adherence to best practices. It helps identify potential issues before they reach production.

## Instructions

### 1. Gather Context

- Identify the files to review using Glob patterns
- Understand the project structure and conventions
- Check for existing linting/formatting rules

### 2. Analyze Code Structure

- Review file organization and module structure
- Check for proper separation of concerns
- Verify naming conventions are consistent

### 3. Check for Common Issues

- Logic errors and edge cases
- Error handling completeness
- Resource management (memory leaks, unclosed handles)
- Thread safety issues in concurrent code
- Input validation gaps

### 4. Evaluate Code Quality

- Readability and clarity
- DRY principle adherence
- SOLID principles compliance
- Appropriate abstraction levels
- Test coverage adequacy

### 5. Performance Review

- Algorithm complexity analysis
- Database query efficiency
- Memory usage patterns
- Caching opportunities

## Best Practices

1. **Be Specific**: Point to exact lines and provide concrete suggestions
2. **Prioritize Issues**: Distinguish between critical bugs and style preferences
3. **Explain Why**: Don't just say what's wrong, explain the reasoning
4. **Suggest Solutions**: Provide alternative implementations when possible
5. **Acknowledge Good Code**: Recognize well-written sections
6. **Consider Context**: Understand the constraints and trade-offs
7. **Be Constructive**: Frame feedback positively and professionally

## Examples

### Example 1: Reviewing a Python Function

```python
# Before Review
def process(data):
    result = []
    for item in data:
        if item['status'] == 'active':
            result.append(item['value'] * 2)
    return result

# Review Comments:
# 1. Function name is too generic - consider 'double_active_values'
# 2. No type hints - add typing for better maintainability
# 3. No docstring explaining purpose and parameters
# 4. No null/empty check on input data
# 5. Could use list comprehension for cleaner code

# After Review
def double_active_values(data: list[dict]) -> list[int]:
    """
    Doubles the values of all active items in the dataset.

    Args:
        data: List of dictionaries with 'status' and 'value' keys

    Returns:
        List of doubled values for active items
    """
    if not data:
        return []
    return [item['value'] * 2 for item in data if item.get('status') == 'active']
```

### Example 2: Security Review Flag

```javascript
// CRITICAL: SQL Injection vulnerability
const query = `SELECT * FROM users WHERE id = ${userId}`;

// Recommendation: Use parameterized queries
const query = "SELECT * FROM users WHERE id = ?";
db.query(query, [userId]);
```

### Example 3: Performance Review

```python
# Issue: O(n*m) complexity due to nested loops with list membership check
for user in users:
    if user.id in active_ids:  # O(n) lookup each time
        process(user)

# Recommendation: Convert to set for O(1) lookups
active_ids_set = set(active_ids)
for user in users:
    if user.id in active_ids_set:
        process(user)
```
