---
name: sql-optimization
description: Analyzes and optimizes SQL queries for better performance, including index design and query rewriting. Trigger keywords: sql, query optimization, slow query, index, explain, performance, database tuning.
allowed-tools: Read, Grep, Glob, Bash
---

# SQL Optimization

## Overview

This skill focuses on analyzing and optimizing SQL queries for improved performance. It covers query analysis, index optimization, execution plan interpretation, and query rewriting strategies.

## Instructions

### 1. Analyze Query Performance

- Identify slow queries from logs
- Run EXPLAIN/EXPLAIN ANALYZE
- Measure query execution time
- Check resource utilization

### 2. Understand Execution Plans

- Identify table scan operations
- Check join algorithms used
- Analyze index usage
- Find bottleneck operations

### 3. Apply Optimizations

- Design appropriate indexes
- Rewrite inefficient queries
- Optimize join order
- Consider denormalization

### 4. Validate Improvements

- Compare before/after metrics
- Test with production-like data
- Verify correctness
- Monitor after deployment

## Best Practices

1. **Index Strategically**: Index columns in WHERE, JOIN, ORDER BY
2. **Avoid SELECT \***: Select only needed columns
3. **Use EXPLAIN**: Always analyze execution plans
4. **Limit Results**: Use pagination for large datasets
5. **Avoid N+1**: Use JOINs or batch queries
6. **Consider Caching**: Cache frequently accessed data
7. **Monitor Continuously**: Track query performance over time

## Examples

### Example 1: Query Optimization with EXPLAIN

```sql
-- Original slow query
SELECT o.*, c.name, c.email
FROM orders o, customers c
WHERE o.customer_id = c.id
AND o.status = 'pending'
AND o.created_at > '2024-01-01'
ORDER BY o.created_at DESC;

-- Step 1: Analyze with EXPLAIN ANALYZE
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT o.*, c.name, c.email
FROM orders o, customers c
WHERE o.customer_id = c.id
AND o.status = 'pending'
AND o.created_at > '2024-01-01'
ORDER BY o.created_at DESC;

-- Output analysis:
-- Seq Scan on orders  (cost=0.00..15420.00 rows=50000)
--   Filter: (status = 'pending' AND created_at > '2024-01-01')
--   Rows Removed by Filter: 450000
-- Problem: Sequential scan on large table!

-- Step 2: Create composite index
CREATE INDEX idx_orders_status_created
ON orders(status, created_at DESC)
WHERE status IN ('pending', 'processing');

-- Step 3: Rewrite with explicit JOIN
SELECT o.id, o.total, o.created_at, c.name, c.email
FROM orders o
INNER JOIN customers c ON o.customer_id = c.id
WHERE o.status = 'pending'
AND o.created_at > '2024-01-01'
ORDER BY o.created_at DESC
LIMIT 100;

-- After optimization:
-- Index Scan using idx_orders_status_created (cost=0.42..125.50 rows=100)
-- 99% reduction in query time!
```

### Example 2: N+1 Query Problem

```sql
-- Problem: N+1 queries
-- Application code:
-- orders = SELECT * FROM orders WHERE user_id = 1
-- for order in orders:
--     items = SELECT * FROM order_items WHERE order_id = order.id

-- Solution: Single query with JOIN
SELECT
    o.id AS order_id,
    o.total,
    o.created_at,
    oi.product_id,
    oi.quantity,
    oi.unit_price,
    p.name AS product_name
FROM orders o
LEFT JOIN order_items oi ON o.id = oi.order_id
LEFT JOIN products p ON oi.product_id = p.id
WHERE o.user_id = 1
ORDER BY o.created_at DESC, oi.id;

-- Alternative: Batch query
SELECT * FROM orders WHERE user_id = 1;
-- Get order IDs: [1, 2, 3, 4, 5]
SELECT * FROM order_items WHERE order_id IN (1, 2, 3, 4, 5);
```

### Example 3: Index Design Strategies

```sql
-- Single column index for equality checks
CREATE INDEX idx_users_email ON users(email);

-- Composite index for multiple conditions
-- Order columns: equality first, then range, then sort
CREATE INDEX idx_orders_user_status_date
ON orders(user_id, status, created_at DESC);

-- Partial index for filtered queries
CREATE INDEX idx_orders_pending
ON orders(created_at DESC)
WHERE status = 'pending';

-- Covering index to avoid table lookups
CREATE INDEX idx_orders_summary
ON orders(user_id, status)
INCLUDE (total, created_at);

-- Expression index for computed conditions
CREATE INDEX idx_users_email_lower ON users(LOWER(email));

-- Check existing indexes
SELECT
    indexname,
    indexdef,
    pg_size_pretty(pg_relation_size(indexname::regclass)) AS size
FROM pg_indexes
WHERE tablename = 'orders';

-- Find unused indexes
SELECT
    schemaname, tablename, indexname,
    idx_scan, idx_tup_read, idx_tup_fetch
FROM pg_stat_user_indexes
WHERE idx_scan = 0
ORDER BY pg_relation_size(indexrelid) DESC;
```

### Example 4: Query Rewriting Patterns

```sql
-- Pattern 1: Replace subquery with JOIN
-- Before
SELECT * FROM orders
WHERE customer_id IN (SELECT id FROM customers WHERE region = 'US');

-- After
SELECT o.* FROM orders o
INNER JOIN customers c ON o.customer_id = c.id
WHERE c.region = 'US';

-- Pattern 2: Use EXISTS instead of IN for large subqueries
-- Before
SELECT * FROM products
WHERE id IN (SELECT product_id FROM order_items);

-- After
SELECT * FROM products p
WHERE EXISTS (
    SELECT 1 FROM order_items oi WHERE oi.product_id = p.id
);

-- Pattern 3: Avoid functions on indexed columns
-- Before (can't use index)
SELECT * FROM users WHERE YEAR(created_at) = 2024;

-- After (uses index)
SELECT * FROM users
WHERE created_at >= '2024-01-01' AND created_at < '2025-01-01';

-- Pattern 4: Optimize pagination
-- Before (slow for large offsets)
SELECT * FROM products ORDER BY id LIMIT 20 OFFSET 10000;

-- After (keyset pagination)
SELECT * FROM products
WHERE id > 10000
ORDER BY id
LIMIT 20;

-- Pattern 5: Batch operations
-- Before (row-by-row)
UPDATE products SET price = price * 1.1 WHERE id = 1;
UPDATE products SET price = price * 1.1 WHERE id = 2;
-- ... repeated 1000 times

-- After (single batch)
UPDATE products SET price = price * 1.1
WHERE id = ANY(ARRAY[1, 2, 3, ..., 1000]);

-- Or use CTE for complex batches
WITH price_updates AS (
    SELECT id, new_price FROM temp_price_updates
)
UPDATE products p
SET price = pu.new_price
FROM price_updates pu
WHERE p.id = pu.id;
```
