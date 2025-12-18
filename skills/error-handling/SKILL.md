---
name: error-handling
description: Comprehensive error handling patterns and strategies. Use when implementing exception handling, error recovery, retry logic, circuit breakers, fallback mechanisms, or designing error hierarchies. Triggers: error, exception, try-catch, retry, fallback, circuit breaker, error propagation, error messages.
---

# Error Handling

## Overview

Error handling is a critical aspect of robust software development. This skill covers error types and hierarchies, recovery strategies, propagation patterns, user-friendly messaging, contextual logging, and language-specific implementations.

## Instructions

### 1. Design Error Hierarchies

Create structured error types that provide clear categorization:

```python
# Python example
class AppError(Exception):
    """Base application error"""
    def __init__(self, message: str, code: str, details: dict = None):
        self.message = message
        self.code = code
        self.details = details or {}
        super().__init__(message)

class ValidationError(AppError):
    """Input validation errors"""
    pass

class NotFoundError(AppError):
    """Resource not found errors"""
    pass

class ServiceError(AppError):
    """External service errors"""
    pass
```

```typescript
// TypeScript example
class AppError extends Error {
  constructor(
    message: string,
    public code: string,
    public details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = this.constructor.name;
  }
}

class ValidationError extends AppError {}
class NotFoundError extends AppError {}
class ServiceError extends AppError {}
```

### 2. Implement Recovery Strategies

#### Retry with Exponential Backoff

```python
import asyncio
from typing import TypeVar, Callable
import random

T = TypeVar('T')

async def retry_with_backoff(
    operation: Callable[[], T],
    max_retries: int = 3,
    base_delay: float = 1.0,
    max_delay: float = 60.0,
    retryable_exceptions: tuple = (ServiceError,)
) -> T:
    """Retry operation with exponential backoff and jitter."""
    for attempt in range(max_retries + 1):
        try:
            return await operation()
        except retryable_exceptions as e:
            if attempt == max_retries:
                raise
            delay = min(base_delay * (2 ** attempt), max_delay)
            jitter = random.uniform(0, delay * 0.1)
            await asyncio.sleep(delay + jitter)
```

#### Circuit Breaker

```python
import time
from enum import Enum
from dataclasses import dataclass

class CircuitState(Enum):
    CLOSED = "closed"
    OPEN = "open"
    HALF_OPEN = "half_open"

@dataclass
class CircuitBreaker:
    failure_threshold: int = 5
    recovery_timeout: float = 30.0
    half_open_max_calls: int = 3

    def __post_init__(self):
        self.state = CircuitState.CLOSED
        self.failure_count = 0
        self.last_failure_time = 0
        self.half_open_calls = 0

    def call(self, operation):
        if self.state == CircuitState.OPEN:
            if time.time() - self.last_failure_time > self.recovery_timeout:
                self.state = CircuitState.HALF_OPEN
                self.half_open_calls = 0
            else:
                raise CircuitOpenError("Circuit is open")

        try:
            result = operation()
            self._on_success()
            return result
        except Exception as e:
            self._on_failure()
            raise

    def _on_success(self):
        if self.state == CircuitState.HALF_OPEN:
            self.half_open_calls += 1
            if self.half_open_calls >= self.half_open_max_calls:
                self.state = CircuitState.CLOSED
        self.failure_count = 0

    def _on_failure(self):
        self.failure_count += 1
        self.last_failure_time = time.time()
        if self.failure_count >= self.failure_threshold:
            self.state = CircuitState.OPEN
```

#### Fallback Pattern

```typescript
async function withFallback<T>(
  primary: () => Promise<T>,
  fallback: () => Promise<T>,
  shouldFallback: (error: Error) => boolean = () => true,
): Promise<T> {
  try {
    return await primary();
  } catch (error) {
    if (shouldFallback(error as Error)) {
      return await fallback();
    }
    throw error;
  }
}

// Usage
const data = await withFallback(
  () => fetchFromPrimaryAPI(),
  () => fetchFromCache(),
  (error) => error instanceof ServiceError,
);
```

### 3. Error Propagation Patterns

#### Wrap and Enrich Errors

```python
def process_order(order_id: str) -> Order:
    try:
        order = fetch_order(order_id)
        validate_order(order)
        return process(order)
    except DatabaseError as e:
        raise ServiceError(
            message="Failed to process order",
            code="ORDER_PROCESSING_FAILED",
            details={"order_id": order_id, "original_error": str(e)}
        ) from e
```

#### Result Types (Rust-style)

```python
from dataclasses import dataclass
from typing import Generic, TypeVar, Union

T = TypeVar('T')
E = TypeVar('E')

@dataclass
class Ok(Generic[T]):
    value: T

@dataclass
class Err(Generic[E]):
    error: E

Result = Union[Ok[T], Err[E]]

def divide(a: float, b: float) -> Result[float, str]:
    if b == 0:
        return Err("Division by zero")
    return Ok(a / b)

# Usage
result = divide(10, 0)
match result:
    case Ok(value):
        print(f"Result: {value}")
    case Err(error):
        print(f"Error: {error}")
```

### 4. User-Friendly Error Messages

```python
ERROR_MESSAGES = {
    "VALIDATION_FAILED": "Please check your input and try again.",
    "NOT_FOUND": "The requested item could not be found.",
    "SERVICE_UNAVAILABLE": "Service is temporarily unavailable. Please try again later.",
    "UNAUTHORIZED": "Please log in to continue.",
    "FORBIDDEN": "You don't have permission to perform this action.",
}

def get_user_message(error: AppError) -> str:
    """Convert internal error to user-friendly message."""
    return ERROR_MESSAGES.get(error.code, "An unexpected error occurred. Please try again.")

def format_error_response(error: AppError, include_details: bool = False) -> dict:
    """Format error for API response."""
    response = {
        "error": {
            "code": error.code,
            "message": get_user_message(error)
        }
    }
    if include_details and error.details:
        response["error"]["details"] = error.details
    return response
```

### 5. Logging Errors with Context

```python
import logging
import traceback
from contextvars import ContextVar

request_id: ContextVar[str] = ContextVar('request_id', default='unknown')

def log_error(error: Exception, context: dict = None):
    """Log error with full context."""
    logger = logging.getLogger(__name__)

    error_context = {
        "request_id": request_id.get(),
        "error_type": type(error).__name__,
        "error_message": str(error),
        "stack_trace": traceback.format_exc(),
        **(context or {})
    }

    if isinstance(error, AppError):
        error_context["error_code"] = error.code
        error_context["error_details"] = error.details

    logger.error(
        f"Error occurred: {error}",
        extra={"structured_data": error_context}
    )
```

## Best Practices

1. **Fail Fast**: Validate inputs early and throw errors immediately rather than continuing with invalid data.

2. **Be Specific**: Create specific error types rather than using generic exceptions. This enables better handling and debugging.

3. **Preserve Context**: When wrapping errors, always preserve the original error chain using mechanisms like `from e` in Python or `cause` in other languages.

4. **Don't Swallow Errors**: Avoid empty catch blocks. At minimum, log the error.

5. **Distinguish Recoverable vs Unrecoverable**: Design your error hierarchy to clearly indicate which errors can be retried.

6. **Use Appropriate Recovery Strategies**:
   - Retry: For transient failures (network timeouts, rate limits)
   - Fallback: When alternatives exist (cache, default values)
   - Circuit Breaker: To prevent cascade failures

7. **Sanitize User-Facing Messages**: Never expose internal error details, stack traces, or sensitive information to users.

8. **Log at Boundaries**: Log errors when they cross system boundaries (API endpoints, service calls).

## Examples

### Complete Error Handling in an API Endpoint

```python
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse

app = FastAPI()

@app.exception_handler(AppError)
async def app_error_handler(request: Request, error: AppError):
    log_error(error, {"path": request.url.path, "method": request.method})

    status_codes = {
        ValidationError: 400,
        NotFoundError: 404,
        ServiceError: 503,
    }

    status_code = status_codes.get(type(error), 500)
    return JSONResponse(
        status_code=status_code,
        content=format_error_response(error)
    )

@app.get("/orders/{order_id}")
async def get_order(order_id: str):
    circuit_breaker = get_circuit_breaker("order_service")

    async def fetch():
        return await order_service.get(order_id)

    try:
        return await retry_with_backoff(
            lambda: circuit_breaker.call(fetch),
            max_retries=3,
            retryable_exceptions=(ServiceError,)
        )
    except CircuitOpenError:
        # Fallback to cache
        cached = await cache.get(f"order:{order_id}")
        if cached:
            return cached
        raise ServiceError(
            message="Order service unavailable",
            code="SERVICE_UNAVAILABLE"
        )
```

### Error Boundary in React

```typescript
import React, { Component, ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Error boundary caught:", error, errorInfo);
    // Send to error tracking service
    errorTracker.captureException(error, { extra: errorInfo });
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback;
    }
    return this.props.children;
  }
}
```
