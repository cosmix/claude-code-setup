---
name: refactoring
description: Restructures existing code to improve readability, maintainability, and performance without changing external behavior. Trigger keywords: refactor, restructure, clean up, improve code, simplify, extract, modernize.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# Refactoring

## Overview

This skill focuses on improving code quality through systematic refactoring techniques. It identifies code smells and applies proven refactoring patterns to enhance maintainability while preserving functionality.

## Instructions

### 1. Identify Refactoring Opportunities

- Search for code smells using pattern matching
- Analyze cyclomatic complexity
- Find duplicated code blocks
- Identify long methods and large classes

### 2. Plan the Refactoring

- List all changes to be made
- Determine dependencies between changes
- Establish rollback points
- Ensure test coverage exists before starting

### 3. Apply Refactoring Patterns

- Extract Method: Break down long functions
- Extract Class: Split large classes
- Rename: Improve naming clarity
- Move Method/Field: Better organize code
- Inline: Remove unnecessary indirection
- Replace Conditional with Polymorphism

### 4. Verify Changes

- Run existing tests after each change
- Check for regressions
- Validate performance hasn't degraded

## Best Practices

1. **Small Steps**: Make incremental changes, not big bang rewrites
2. **Test First**: Ensure tests exist before refactoring
3. **One Thing at a Time**: Focus on single refactoring per commit
4. **Preserve Behavior**: External behavior must remain unchanged
5. **Keep It Working**: Code should pass tests after each step
6. **Document Intent**: Explain why refactoring was needed

## Examples

### Example 1: Extract Method

```python
# Before: Long method with multiple responsibilities
def process_order(order):
    # Validate order
    if not order.items:
        raise ValueError("Empty order")
    if order.total < 0:
        raise ValueError("Invalid total")

    # Calculate discount
    discount = 0
    if order.customer.is_premium:
        discount = order.total * 0.1
    if order.total > 1000:
        discount += order.total * 0.05

    # Apply discount and save
    order.final_total = order.total - discount
    order.save()

# After: Extracted methods with single responsibility
def process_order(order):
    validate_order(order)
    discount = calculate_discount(order)
    finalize_order(order, discount)

def validate_order(order):
    if not order.items:
        raise ValueError("Empty order")
    if order.total < 0:
        raise ValueError("Invalid total")

def calculate_discount(order) -> float:
    discount = 0
    if order.customer.is_premium:
        discount = order.total * 0.1
    if order.total > 1000:
        discount += order.total * 0.05
    return discount

def finalize_order(order, discount: float):
    order.final_total = order.total - discount
    order.save()
```

### Example 2: Replace Magic Numbers with Constants

```javascript
// Before
if (response.status === 200) {
  setTimeout(retry, 3000);
}

// After
const HTTP_OK = 200;
const RETRY_DELAY_MS = 3000;

if (response.status === HTTP_OK) {
  setTimeout(retry, RETRY_DELAY_MS);
}
```

### Example 3: Replace Nested Conditionals with Guard Clauses

```python
# Before
def get_payment_amount(employee):
    if employee.is_active:
        if employee.is_full_time:
            if employee.tenure > 5:
                return employee.salary * 1.1
            else:
                return employee.salary
        else:
            return employee.hourly_rate * employee.hours
    else:
        return 0

# After
def get_payment_amount(employee):
    if not employee.is_active:
        return 0

    if not employee.is_full_time:
        return employee.hourly_rate * employee.hours

    if employee.tenure > 5:
        return employee.salary * 1.1

    return employee.salary
```
