---
name: database-design
description: Designs database schemas, relationships, indexes, and migrations for relational and NoSQL databases. Trigger keywords: database design, schema, migration, ERD, normalization, index, foreign key, table design.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# Database Design

## Overview

This skill focuses on designing efficient, scalable, and maintainable database schemas. It covers relational databases (PostgreSQL, MySQL), NoSQL databases (MongoDB, Redis), and data modeling best practices.

## Instructions

### 1. Understand Data Requirements

- Identify entities and their attributes
- Map relationships between entities
- Determine data access patterns
- Estimate data volumes and growth

### 2. Design Schema

- Normalize data appropriately (typically 3NF)
- Define primary keys and foreign keys
- Choose appropriate data types
- Plan for NULL handling

### 3. Optimize for Performance

- Design indexes for query patterns
- Consider denormalization where needed
- Plan partitioning strategy for large tables
- Design for concurrent access

### 4. Plan Migrations

- Create reversible migrations
- Handle data transformations
- Plan for zero-downtime deployments
- Version control schema changes

## Best Practices

1. **Choose Appropriate Types**: Use correct data types for storage efficiency
2. **Index Wisely**: Index columns used in WHERE, JOIN, and ORDER BY
3. **Normalize First**: Start normalized, denormalize for performance
4. **Use Constraints**: Enforce data integrity at database level
5. **Plan for Scale**: Consider sharding and replication early
6. **Document Schemas**: Maintain ERD and data dictionary
7. **Test Migrations**: Always test migrations on production-like data

## Examples

### Example 1: E-Commerce Schema (PostgreSQL)

```sql
-- Users table with proper constraints
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,

    CONSTRAINT email_format CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$')
);

-- Products with proper indexing
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sku VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price DECIMAL(10, 2) NOT NULL CHECK (price >= 0),
    stock_quantity INTEGER NOT NULL DEFAULT 0 CHECK (stock_quantity >= 0),
    category_id UUID REFERENCES categories(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_products_category ON products(category_id);
CREATE INDEX idx_products_price ON products(price);
CREATE INDEX idx_products_name_search ON products USING gin(to_tsvector('english', name));

-- Orders with proper relationships
CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    total_amount DECIMAL(10, 2) NOT NULL,
    shipping_address JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_status CHECK (status IN ('pending', 'paid', 'shipped', 'delivered', 'cancelled'))
);

CREATE INDEX idx_orders_user ON orders(user_id);
CREATE INDEX idx_orders_status ON orders(status);
CREATE INDEX idx_orders_created ON orders(created_at DESC);

-- Order items junction table
CREATE TABLE order_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price DECIMAL(10, 2) NOT NULL,

    UNIQUE(order_id, product_id)
);
```

### Example 2: Migration Script

```sql
-- Migration: Add customer loyalty program
-- Version: 20240115_001

BEGIN;

-- Add loyalty tier to users
ALTER TABLE users
ADD COLUMN loyalty_tier VARCHAR(20) DEFAULT 'bronze',
ADD COLUMN loyalty_points INTEGER DEFAULT 0;

-- Add constraint for valid tiers
ALTER TABLE users
ADD CONSTRAINT valid_loyalty_tier
CHECK (loyalty_tier IN ('bronze', 'silver', 'gold', 'platinum'));

-- Create points history table
CREATE TABLE loyalty_points_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    points_change INTEGER NOT NULL,
    reason VARCHAR(100) NOT NULL,
    reference_type VARCHAR(50),
    reference_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loyalty_history_user ON loyalty_points_history(user_id);
CREATE INDEX idx_loyalty_history_created ON loyalty_points_history(created_at DESC);

COMMIT;

-- Rollback script (save separately)
-- BEGIN;
-- DROP TABLE IF EXISTS loyalty_points_history;
-- ALTER TABLE users DROP COLUMN IF EXISTS loyalty_points;
-- ALTER TABLE users DROP COLUMN IF EXISTS loyalty_tier;
-- COMMIT;
```

### Example 3: MongoDB Document Design

```javascript
// User document with embedded addresses
{
  _id: ObjectId("..."),
  email: "user@example.com",
  profile: {
    name: "John Doe",
    avatar_url: "https://..."
  },
  addresses: [
    {
      type: "shipping",
      street: "123 Main St",
      city: "Boston",
      state: "MA",
      zip: "02101",
      is_default: true
    }
  ],
  preferences: {
    newsletter: true,
    notifications: {
      email: true,
      push: false
    }
  },
  created_at: ISODate("2024-01-15T10:00:00Z")
}

// Indexes
db.users.createIndex({ email: 1 }, { unique: true });
db.users.createIndex({ "addresses.zip": 1 });
db.users.createIndex({ created_at: -1 });
```
