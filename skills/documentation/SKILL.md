---
name: documentation
description: Creates and maintains technical documentation including API docs, README files, architecture docs, and inline code comments. Trigger keywords: document, docs, readme, api docs, jsdoc, docstring, architecture doc.
allowed-tools: Read, Grep, Glob, Edit, Write
---

# Documentation

## Overview

This skill focuses on creating clear, comprehensive, and maintainable documentation. It covers code documentation, API references, architectural documentation, and user guides.

## Instructions

### 1. Assess Documentation Needs

- Identify target audience (developers, users, operators)
- Determine documentation types needed
- Review existing documentation
- Understand the codebase structure

### 2. Document Code

- Add docstrings/JSDoc to functions and classes
- Include type information
- Document parameters, return values, and exceptions
- Add usage examples

### 3. Create API Documentation

- Document all endpoints/methods
- Include request/response formats
- Provide authentication details
- Show error responses

### 4. Write Architectural Docs

- System overview and components
- Data flow diagrams
- Integration points
- Deployment architecture

## Best Practices

1. **Write for Your Audience**: Match complexity to reader expertise
2. **Keep It Current**: Update docs when code changes
3. **Use Examples**: Show, don't just tell
4. **Be Concise**: Remove unnecessary words
5. **Structure Consistently**: Use templates and patterns
6. **Include the Why**: Explain decisions, not just facts
7. **Make It Searchable**: Use clear headings and keywords

## Examples

### Example 1: Python Docstring (Google Style)

```python
def calculate_shipping_cost(
    weight: float,
    destination: str,
    express: bool = False
) -> Decimal:
    """Calculate shipping cost based on package weight and destination.

    Applies tiered pricing based on weight brackets and adds surcharges
    for international destinations and express delivery.

    Args:
        weight: Package weight in kilograms. Must be positive.
        destination: ISO 3166-1 alpha-2 country code (e.g., 'US', 'GB').
        express: If True, uses express delivery (2-3 days).
                 Default is standard delivery (5-7 days).

    Returns:
        The calculated shipping cost in USD as a Decimal.

    Raises:
        ValueError: If weight is not positive.
        InvalidDestinationError: If country code is not recognized.

    Example:
        >>> calculate_shipping_cost(2.5, 'US')
        Decimal('12.50')
        >>> calculate_shipping_cost(2.5, 'GB', express=True)
        Decimal('45.00')
    """
```

### Example 2: TypeScript JSDoc

````typescript
/**
 * Manages user authentication and session handling.
 *
 * @example
 * ```typescript
 * const auth = new AuthService(config);
 * const token = await auth.login('user@example.com', 'password');
 * const user = await auth.validateToken(token);
 * ```
 */
class AuthService {
  /**
   * Authenticates a user with email and password.
   *
   * @param email - User's email address
   * @param password - User's password (will be hashed)
   * @returns JWT access token valid for 24 hours
   * @throws {InvalidCredentialsError} When email/password combination is invalid
   * @throws {AccountLockedError} When account is locked due to failed attempts
   */
  async login(email: string, password: string): Promise<string> {
    // Implementation
  }
}
````

### Example 3: API Endpoint Documentation

````markdown
## Create User

Creates a new user account.

**Endpoint:** `POST /api/v1/users`

**Authentication:** Required (Admin role)

**Request Body:**
| Field | Type | Required | Description |
|-----------|--------|----------|--------------------------------|
| email | string | Yes | Valid email address |
| name | string | Yes | Full name (2-100 characters) |
| role | string | No | User role. Default: "member" |

**Example Request:**

```json
{
  "email": "jane@example.com",
  "name": "Jane Smith",
  "role": "admin"
}
```
````

**Success Response (201 Created):**

```json
{
  "id": "usr_abc123",
  "email": "jane@example.com",
  "name": "Jane Smith",
  "role": "admin",
  "createdAt": "2024-01-15T10:30:00Z"
}
```

**Error Responses:**
| Status | Code | Description |
|--------|-------------------|---------------------------------|
| 400 | INVALID_EMAIL | Email format is invalid |
| 400 | NAME_TOO_SHORT | Name must be at least 2 chars |
| 409 | EMAIL_EXISTS | Email already registered |
| 403 | FORBIDDEN | Insufficient permissions |

```

```
