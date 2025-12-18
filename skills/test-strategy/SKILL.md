---
name: test-strategy
description: Comprehensive test strategy guidance including test pyramid design, coverage goals, test categorization, CI/CD integration, and risk-based prioritization. Use when planning testing approaches, setting up test infrastructure, or optimizing test suites. Trigger keywords: test strategy, test pyramid, coverage goals, what to test, test organization, CI/CD testing, test prioritization, testing approach.
---

# Test Strategy

## Overview

Test strategy defines how to approach testing for a project, balancing thoroughness with efficiency. A well-designed strategy ensures critical functionality is covered while avoiding over-testing trivial code. This skill covers the test pyramid, coverage metrics, test categorization, and integration with CI/CD pipelines.

## Instructions

### 1. Design the Test Pyramid

Structure tests in layers with appropriate ratios:

```
         /\
        /  \        E2E Tests (5-10%)
       /----\       - Critical user journeys
      /      \      - Cross-system integration
     /--------\     Integration Tests (15-25%)
    /          \    - API contracts
   /------------\   - Database interactions
  /              \  - Service boundaries
 /----------------\ Unit Tests (65-80%)
                    - Business logic
                    - Pure functions
                    - Edge cases
```

**Recommended Ratios:**

- Unit tests: 65-80% of test suite
- Integration tests: 15-25%
- E2E tests: 5-10%

### 2. Set Coverage Goals

**Coverage Targets by Component Type:**

| Component Type | Line Coverage | Branch Coverage | Notes                          |
| -------------- | ------------- | --------------- | ------------------------------ |
| Business Logic | 90%+          | 85%+            | Critical paths fully covered   |
| API Handlers   | 80%+          | 75%+            | All endpoints tested           |
| Utilities      | 95%+          | 90%+            | Pure functions easily testable |
| UI Components  | 70%+          | 60%+            | Focus on behavior over markup  |
| Infrastructure | 60%+          | 50%+            | Integration tests preferred    |

**Coverage Anti-patterns to Avoid:**

- Chasing 100% coverage for coverage's sake
- Testing getters/setters without logic
- Testing framework or library code
- Writing tests that don't verify behavior

### 3. Decide What to Test vs What Not to Test

**Always Test:**

- Business logic and domain rules
- Input validation and error handling
- Security-sensitive operations
- Data transformations
- State transitions
- Edge cases and boundary conditions
- Regression scenarios from bug fixes

**Consider Not Testing:**

- Simple pass-through functions
- Framework-generated code
- Third-party library internals
- Trivial getters/setters
- Configuration constants
- Logging statements (unless critical)

**Test Smell Detection:**

```typescript
// BAD: Testing trivial code
test("getter returns value", () => {
  const user = new User("John");
  expect(user.getName()).toBe("John");
});

// GOOD: Testing meaningful behavior
test("user cannot change name to empty string", () => {
  const user = new User("John");
  expect(() => user.setName("")).toThrow(ValidationError);
});
```

### 4. Categorize and Organize Tests

**Directory Structure:**

```
tests/
├── unit/
│   ├── services/
│   ├── models/
│   └── utils/
├── integration/
│   ├── api/
│   ├── database/
│   └── external-services/
├── e2e/
│   ├── flows/
│   └── pages/
├── fixtures/
│   ├── factories/
│   └── mocks/
└── helpers/
    ├── setup.ts
    └── assertions.ts
```

**Test Tagging System:**

```typescript
// Jest example with tags
describe("[unit][fast] UserService", () => {});
describe("[integration][slow] DatabaseRepository", () => {});
describe("[e2e][critical] CheckoutFlow", () => {});

// Run specific categories
// npm test -- --grep="\[unit\]"
// npm test -- --grep="\[critical\]"
```

**Naming Conventions:**

```
[ComponentName].[scenario].[expected_result].test.ts

Examples:
UserService.createUser.returnsNewUser.test.ts
PaymentProcessor.invalidCard.throwsPaymentError.test.ts
```

### 5. Integrate with CI/CD

**Pipeline Stage Configuration:**

```yaml
# .github/workflows/test.yml
name: Test Pipeline

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run Unit Tests
        run: npm test -- --grep="\[unit\]" --coverage
      - name: Upload Coverage
        uses: codecov/codecov-action@v3

  integration-tests:
    runs-on: ubuntu-latest
    needs: unit-tests
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: test
    steps:
      - uses: actions/checkout@v4
      - name: Run Integration Tests
        run: npm test -- --grep="\[integration\]"

  e2e-tests:
    runs-on: ubuntu-latest
    needs: integration-tests
    steps:
      - uses: actions/checkout@v4
      - name: Run E2E Tests
        run: npm run test:e2e
```

**CI Test Optimization:**

- Run unit tests first (fast feedback)
- Parallelize test suites
- Cache dependencies and build artifacts
- Use test splitting for large suites
- Fail fast on critical tests

### 6. Risk-Based Test Prioritization

**Risk Matrix for Prioritization:**

| Impact ↓ / Likelihood → | Low             | Medium          | High            |
| ----------------------- | --------------- | --------------- | --------------- |
| High                    | Medium Priority | High Priority   | Critical        |
| Medium                  | Low Priority    | Medium Priority | High Priority   |
| Low                     | Skip/Manual     | Low Priority    | Medium Priority |

**Risk Factors to Consider:**

- **Business Impact:** Revenue, user trust, legal compliance
- **Complexity:** Code complexity, integration points
- **Change Frequency:** Actively developed areas
- **Historical Bugs:** Components with bug history
- **Dependencies:** Critical external services

**Prioritized Test Categories:**

1. **Critical (P0):** Run on every commit
   - Authentication/authorization
   - Payment processing
   - Data integrity

2. **High (P1):** Run on PR merge
   - Core business workflows
   - API contract tests

3. **Medium (P2):** Run nightly
   - Edge cases
   - Performance tests

4. **Low (P3):** Run weekly
   - Backward compatibility
   - Deprecated feature coverage

## Best Practices

1. **Test Behavior, Not Implementation**
   - Tests should verify outcomes, not internal mechanics
   - Refactoring should not break tests if behavior unchanged

2. **Keep Tests Independent**
   - No shared mutable state between tests
   - Each test sets up its own context
   - Tests can run in any order

3. **Use Test Doubles Appropriately**
   - Stubs for providing test data
   - Mocks for verifying interactions
   - Fakes for complex dependencies
   - Real implementations when feasible

4. **Maintain Test Quality**
   - Apply same code quality standards to tests
   - Refactor test code for readability
   - Remove obsolete tests promptly

5. **Fast Feedback Loop**
   - Optimize for quick local test runs
   - Use watch mode during development
   - Prioritize fast tests in CI

6. **Document Test Intent**
   - Clear test names describe behavior
   - Add comments for non-obvious setup
   - Link tests to requirements/tickets

## Examples

### Example: Feature Test Strategy Document

```markdown
# Feature: User Registration

## Risk Assessment

- Business Impact: HIGH (user acquisition)
- Complexity: MEDIUM (email validation, password rules)
- Change Frequency: LOW (stable feature)

## Test Coverage Plan

### Unit Tests (P0)

- [ ] Email format validation
- [ ] Password strength requirements
- [ ] Username uniqueness check logic
- [ ] Profile data sanitization

### Integration Tests (P1)

- [ ] Database user creation
- [ ] Email service integration
- [ ] Duplicate email handling

### E2E Tests (P0)

- [ ] Happy path: complete registration flow
- [ ] Error path: duplicate email shows error

## Coverage Targets

- Line coverage: 85%
- Branch coverage: 80%
- Critical paths: 100%
```

### Example: Test Organization Configuration

```javascript
// jest.config.js
module.exports = {
  projects: [
    {
      displayName: "unit",
      testMatch: ["<rootDir>/tests/unit/**/*.test.ts"],
      setupFilesAfterEnv: ["<rootDir>/tests/helpers/unit-setup.ts"],
    },
    {
      displayName: "integration",
      testMatch: ["<rootDir>/tests/integration/**/*.test.ts"],
      setupFilesAfterEnv: ["<rootDir>/tests/helpers/integration-setup.ts"],
      globalSetup: "<rootDir>/tests/helpers/db-setup.ts",
      globalTeardown: "<rootDir>/tests/helpers/db-teardown.ts",
    },
  ],
  coverageThreshold: {
    global: {
      branches: 75,
      functions: 80,
      lines: 80,
      statements: 80,
    },
    "./src/services/": {
      branches: 90,
      lines: 90,
    },
  },
};
```

### Example: Risk-Based Test Selection Script

```typescript
// scripts/select-tests.ts
interface TestFile {
  path: string;
  priority: "P0" | "P1" | "P2" | "P3";
  tags: string[];
}

function selectTestsForPipeline(
  context: "commit" | "pr" | "nightly" | "weekly",
): TestFile[] {
  const allTests = getTestManifest();

  const priorityMap = {
    commit: ["P0"],
    pr: ["P0", "P1"],
    nightly: ["P0", "P1", "P2"],
    weekly: ["P0", "P1", "P2", "P3"],
  };

  return allTests.filter((test) =>
    priorityMap[context].includes(test.priority),
  );
}
```
