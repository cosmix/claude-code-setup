---
name: python
description: Python language expertise for writing idiomatic, production-quality Python code. Use for Python development, type hints, async patterns, testing with pytest, packaging, and following PEP 8 standards. Triggers: python, py, pytest, pep8, typing, asyncio, poetry, uv, pyproject.
---

# Python Language Expertise

## Overview

This skill provides guidance for writing idiomatic, maintainable, and production-quality Python code. It covers modern Python practices including type hints, async programming, testing patterns, and proper packaging.

## Key Concepts

### Type Hints (typing module)

```python
from typing import Optional, Union, List, Dict, Callable, TypeVar, Generic
from collections.abc import Sequence, Mapping, Iterator

T = TypeVar('T')
K = TypeVar('K')
V = TypeVar('V')

def process_items(items: Sequence[T], transform: Callable[[T], T]) -> list[T]:
    return [transform(item) for item in items]

class Repository(Generic[T]):
    def __init__(self) -> None:
        self._items: dict[str, T] = {}

    def get(self, key: str) -> T | None:
        return self._items.get(key)

    def set(self, key: str, value: T) -> None:
        self._items[key] = value
```

### Async/Await Patterns

```python
import asyncio
from typing import AsyncIterator

async def fetch_data(url: str) -> dict:
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.json()

async def process_batch(urls: list[str]) -> list[dict]:
    tasks = [fetch_data(url) for url in urls]
    return await asyncio.gather(*tasks, return_exceptions=True)

async def stream_items(source: AsyncIterator[bytes]) -> AsyncIterator[dict]:
    async for chunk in source:
        yield json.loads(chunk)
```

### Context Managers

```python
from contextlib import contextmanager, asynccontextmanager
from typing import Iterator, AsyncIterator

@contextmanager
def managed_resource(name: str) -> Iterator[Resource]:
    resource = Resource(name)
    try:
        resource.acquire()
        yield resource
    finally:
        resource.release()

@asynccontextmanager
async def async_transaction(db: Database) -> AsyncIterator[Transaction]:
    tx = await db.begin()
    try:
        yield tx
        await tx.commit()
    except Exception:
        await tx.rollback()
        raise
```

### Decorators

```python
from functools import wraps
from typing import Callable, ParamSpec, TypeVar

P = ParamSpec('P')
R = TypeVar('R')

def retry(max_attempts: int = 3) -> Callable[[Callable[P, R]], Callable[P, R]]:
    def decorator(func: Callable[P, R]) -> Callable[P, R]:
        @wraps(func)
        def wrapper(*args: P.args, **kwargs: P.kwargs) -> R:
            last_exception: Exception | None = None
            for attempt in range(max_attempts):
                try:
                    return func(*args, **kwargs)
                except Exception as e:
                    last_exception = e
            raise last_exception
        return wrapper
    return decorator
```

### Generators

```python
from typing import Generator, Iterator

def paginate(items: Sequence[T], page_size: int) -> Generator[list[T], None, None]:
    for i in range(0, len(items), page_size):
        yield list(items[i:i + page_size])

def read_chunks(file_path: str, chunk_size: int = 8192) -> Iterator[bytes]:
    with open(file_path, 'rb') as f:
        while chunk := f.read(chunk_size):
            yield chunk
```

## Best Practices

### PEP 8 Compliance

- Use 4 spaces for indentation (never tabs)
- Maximum line length of 88 characters (black default) or 79 (strict PEP 8)
- Use snake_case for functions and variables, PascalCase for classes
- Two blank lines before top-level definitions, one blank line between methods
- Imports at the top: standard library, third-party, local (separated by blank lines)

### Modern Python Features (3.10+)

```python
# Structural pattern matching
match command:
    case {"action": "create", "name": str(name)}:
        create_resource(name)
    case {"action": "delete", "id": int(id_)}:
        delete_resource(id_)
    case _:
        raise ValueError("Unknown command")

# Union types with |
def process(value: int | str | None) -> str:
    ...

# Self type for fluent interfaces
from typing import Self

class Builder:
    def with_name(self, name: str) -> Self:
        self._name = name
        return self
```

### Packaging with pyproject.toml

```toml
[project]
name = "mypackage"
version = "0.1.0"
description = "A sample package"
requires-python = ">=3.11"
dependencies = [
    "httpx>=0.25.0",
    "pydantic>=2.0.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "pytest-asyncio>=0.21.0",
    "mypy>=1.0.0",
    "ruff>=0.1.0",
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.ruff]
line-length = 88
target-version = "py311"

[tool.mypy]
strict = true
python_version = "3.11"
```

## Common Patterns

### Dataclasses and Pydantic Models

```python
from dataclasses import dataclass, field
from pydantic import BaseModel, Field, field_validator

@dataclass
class Config:
    host: str
    port: int = 8080
    tags: list[str] = field(default_factory=list)

class UserCreate(BaseModel):
    email: str = Field(..., min_length=5)
    name: str = Field(..., max_length=100)

    @field_validator('email')
    @classmethod
    def validate_email(cls, v: str) -> str:
        if '@' not in v:
            raise ValueError('Invalid email')
        return v.lower()
```

### Testing with pytest

```python
import pytest
from unittest.mock import AsyncMock, patch

@pytest.fixture
def client() -> TestClient:
    return TestClient(app)

@pytest.fixture
async def db_session() -> AsyncIterator[AsyncSession]:
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    async with AsyncSession(engine) as session:
        yield session

class TestUserService:
    @pytest.mark.asyncio
    async def test_create_user(self, db_session: AsyncSession) -> None:
        service = UserService(db_session)
        user = await service.create(name="test", email="test@example.com")
        assert user.id is not None

    @pytest.mark.parametrize("email,valid", [
        ("user@example.com", True),
        ("invalid", False),
        ("", False),
    ])
    def test_email_validation(self, email: str, valid: bool) -> None:
        if valid:
            User(email=email, name="test")
        else:
            with pytest.raises(ValueError):
                User(email=email, name="test")

    @patch("mymodule.external_api")
    async def test_with_mock(self, mock_api: AsyncMock) -> None:
        mock_api.fetch.return_value = {"status": "ok"}
        result = await process_with_api()
        mock_api.fetch.assert_called_once()
```

## Anti-Patterns

### Avoid These Practices

```python
# BAD: Mutable default arguments
def append_to(item, target=[]):  # Bug: shared list across calls
    target.append(item)
    return target

# GOOD: Use None and create new list
def append_to(item, target=None):
    if target is None:
        target = []
    target.append(item)
    return target

# BAD: Bare except clauses
try:
    risky_operation()
except:  # Catches SystemExit, KeyboardInterrupt too
    pass

# GOOD: Catch specific exceptions
try:
    risky_operation()
except (ValueError, RuntimeError) as e:
    logger.error(f"Operation failed: {e}")

# BAD: String formatting with + for complex strings
message = "User " + name + " has " + str(count) + " items"

# GOOD: f-strings
message = f"User {name} has {count} items"

# BAD: Checking type with type()
if type(obj) == list:
    ...

# GOOD: Use isinstance for type checking
if isinstance(obj, list):
    ...

# BAD: Not using context managers for resources
f = open("file.txt")
data = f.read()
f.close()

# GOOD: Always use context managers
with open("file.txt") as f:
    data = f.read()

# BAD: Global mutable state
_cache = {}

def get_cached(key):
    return _cache.get(key)

# GOOD: Encapsulate state in classes or use dependency injection
class Cache:
    def __init__(self):
        self._store: dict[str, Any] = {}

    def get(self, key: str) -> Any | None:
        return self._store.get(key)
```
