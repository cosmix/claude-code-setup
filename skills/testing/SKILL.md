---
name: testing
description: Creates comprehensive test suites including unit tests, integration tests, and end-to-end tests. Trigger keywords: test, testing, unit test, integration test, e2e, coverage, TDD, mock, fixture.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# Testing

## Overview

This skill helps create robust test suites that ensure code correctness and prevent regressions. It covers unit testing, integration testing, and end-to-end testing strategies across different frameworks and languages.

## Instructions

### 1. Analyze Code to Test

- Identify public interfaces and APIs
- Map out dependencies and side effects
- Find edge cases and boundary conditions
- Understand expected behaviors

### 2. Design Test Strategy

- Determine appropriate test types (unit/integration/e2e)
- Plan test coverage targets
- Identify mocking requirements
- Set up test fixtures and data

### 3. Write Tests Following AAA Pattern

- **Arrange**: Set up test data and conditions
- **Act**: Execute the code under test
- **Assert**: Verify expected outcomes

### 4. Handle Special Cases

- Async operations and timeouts
- Error conditions and exceptions
- External service dependencies
- Database interactions

## Best Practices

1. **Test One Thing**: Each test should verify a single behavior
2. **Descriptive Names**: Test names should describe the scenario and expected outcome
3. **Independent Tests**: Tests should not depend on each other
4. **Fast Execution**: Unit tests should run in milliseconds
5. **Deterministic**: Same input should always produce same result
6. **Test Edge Cases**: Include boundary conditions and error paths
7. **Avoid Test Logic**: Tests should be simple assertions, not algorithms

## Examples

### Example 1: Python Unit Test with pytest

```python
import pytest
from decimal import Decimal
from shopping_cart import ShoppingCart, Item

class TestShoppingCart:
    @pytest.fixture
    def cart(self):
        return ShoppingCart()

    @pytest.fixture
    def sample_item(self):
        return Item(name="Widget", price=Decimal("19.99"), quantity=1)

    def test_empty_cart_has_zero_total(self, cart):
        assert cart.total == Decimal("0")

    def test_add_item_increases_total(self, cart, sample_item):
        cart.add(sample_item)
        assert cart.total == Decimal("19.99")

    def test_add_multiple_quantities(self, cart):
        item = Item(name="Gadget", price=Decimal("10.00"), quantity=3)
        cart.add(item)
        assert cart.total == Decimal("30.00")

    def test_remove_item_decreases_total(self, cart, sample_item):
        cart.add(sample_item)
        cart.remove(sample_item.name)
        assert cart.total == Decimal("0")

    def test_remove_nonexistent_item_raises_error(self, cart):
        with pytest.raises(KeyError):
            cart.remove("NonexistentItem")

    def test_apply_discount_reduces_total(self, cart, sample_item):
        cart.add(sample_item)
        cart.apply_discount(percent=10)
        assert cart.total == Decimal("17.99")
```

### Example 2: JavaScript Integration Test

```javascript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { createServer } from "../server";
import { db } from "../database";

describe("User API Integration", () => {
  let server;

  beforeEach(async () => {
    server = await createServer();
    await db.migrate.latest();
    await db.seed.run();
  });

  afterEach(async () => {
    await db.migrate.rollback();
    await server.close();
  });

  it("creates a new user and returns 201", async () => {
    const response = await server.inject({
      method: "POST",
      url: "/api/users",
      payload: {
        email: "new@example.com",
        name: "New User",
      },
    });

    expect(response.statusCode).toBe(201);
    expect(response.json()).toMatchObject({
      email: "new@example.com",
      name: "New User",
    });
  });

  it("returns 400 for invalid email", async () => {
    const response = await server.inject({
      method: "POST",
      url: "/api/users",
      payload: {
        email: "invalid-email",
        name: "Test User",
      },
    });

    expect(response.statusCode).toBe(400);
    expect(response.json().error).toContain("email");
  });
});
```

### Example 3: Mocking External Dependencies

```python
from unittest.mock import Mock, patch
import pytest
from payment_processor import PaymentProcessor

class TestPaymentProcessor:
    @patch('payment_processor.stripe')
    def test_successful_payment(self, mock_stripe):
        # Arrange
        mock_stripe.Charge.create.return_value = Mock(
            id='ch_123',
            status='succeeded'
        )
        processor = PaymentProcessor()

        # Act
        result = processor.charge(amount=1000, token='tok_visa')

        # Assert
        assert result.success is True
        assert result.charge_id == 'ch_123'
        mock_stripe.Charge.create.assert_called_once_with(
            amount=1000,
            currency='usd',
            source='tok_visa'
        )

    @patch('payment_processor.stripe')
    def test_failed_payment_raises_exception(self, mock_stripe):
        mock_stripe.Charge.create.side_effect = Exception("Card declined")
        processor = PaymentProcessor()

        with pytest.raises(PaymentError) as exc_info:
            processor.charge(amount=1000, token='tok_declined')

        assert "Card declined" in str(exc_info.value)
```
