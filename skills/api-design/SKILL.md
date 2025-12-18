---
name: api-design
description: Designs RESTful APIs, GraphQL schemas, and RPC interfaces following best practices for consistency, usability, and scalability. Trigger keywords: api design, rest api, graphql, openapi, swagger, endpoint, schema design.
allowed-tools: Read, Grep, Glob, Edit, Write
---

# API Design

## Overview

This skill guides the design of well-structured APIs that are intuitive, consistent, and scalable. It covers REST, GraphQL, and gRPC patterns with focus on developer experience and long-term maintainability.

## Instructions

### 1. Understand Requirements

- Identify API consumers and use cases
- Define resource models and relationships
- Determine authentication/authorization needs
- Plan for versioning strategy

### 2. Design Resource Structure

- Define clear resource hierarchies
- Establish naming conventions
- Plan URL patterns
- Design request/response schemas

### 3. Define Operations

- Map CRUD operations to HTTP methods
- Design query parameters for filtering/sorting
- Plan pagination approach
- Define error response formats

### 4. Document the API

- Create OpenAPI/Swagger specification
- Include examples for all endpoints
- Document authentication flows
- Provide SDK usage examples

## Best Practices

1. **Use Nouns for Resources**: `/users`, not `/getUsers`
2. **Consistent Naming**: Use snake_case or camelCase consistently
3. **Proper HTTP Methods**: GET (read), POST (create), PUT/PATCH (update), DELETE (remove)
4. **Meaningful Status Codes**: 200, 201, 400, 401, 403, 404, 500
5. **Version Your API**: `/v1/users` or `Accept: application/vnd.api.v1+json`
6. **HATEOAS**: Include links to related resources
7. **Idempotency**: PUT and DELETE should be idempotent

## Examples

### Example 1: RESTful Resource Design

```yaml
# OpenAPI 3.0 Specification
openapi: 3.0.0
info:
  title: E-Commerce API
  version: 1.0.0

paths:
  /products:
    get:
      summary: List all products
      parameters:
        - name: category
          in: query
          schema:
            type: string
        - name: min_price
          in: query
          schema:
            type: number
        - name: page
          in: query
          schema:
            type: integer
            default: 1
        - name: per_page
          in: query
          schema:
            type: integer
            default: 20
            maximum: 100
      responses:
        "200":
          description: Paginated list of products
          content:
            application/json:
              schema:
                type: object
                properties:
                  data:
                    type: array
                    items:
                      $ref: "#/components/schemas/Product"
                  pagination:
                    $ref: "#/components/schemas/Pagination"

    post:
      summary: Create a new product
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/ProductInput"
      responses:
        "201":
          description: Product created
        "400":
          description: Validation error

  /products/{id}:
    get:
      summary: Get a specific product
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        "200":
          description: Product details
        "404":
          description: Product not found
```

### Example 2: GraphQL Schema Design

```graphql
type Query {
  """
  Fetch a user by ID
  """
  user(id: ID!): User

  """
  List users with optional filtering
  """
  users(filter: UserFilter, first: Int = 20, after: String): UserConnection!
}

type Mutation {
  """
  Create a new user account
  """
  createUser(input: CreateUserInput!): CreateUserPayload!

  """
  Update an existing user
  """
  updateUser(id: ID!, input: UpdateUserInput!): UpdateUserPayload!
}

type User {
  id: ID!
  email: String!
  name: String!
  createdAt: DateTime!
  orders(first: Int, after: String): OrderConnection!
}

input CreateUserInput {
  email: String!
  name: String!
  password: String!
}

type CreateUserPayload {
  user: User
  errors: [UserError!]!
}

type UserError {
  field: String
  message: String!
  code: UserErrorCode!
}
```

### Example 3: Error Response Design

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Request validation failed",
    "details": [
      {
        "field": "email",
        "code": "INVALID_FORMAT",
        "message": "Email must be a valid email address"
      },
      {
        "field": "age",
        "code": "OUT_OF_RANGE",
        "message": "Age must be between 18 and 120"
      }
    ],
    "request_id": "req_abc123xyz",
    "documentation_url": "https://api.example.com/docs/errors#VALIDATION_ERROR"
  }
}
```
