---
name: code-migration
description: Strategies and patterns for safe code migrations and upgrades. Use when upgrading frameworks, migrating between technologies, handling deprecations, or planning incremental migrations. Triggers: migration, upgrade, deprecation, framework migration, version upgrade, legacy code, modernization, refactoring, feature flags, rollback.
---

# Code Migration

## Overview

Code migration involves moving codebases between versions, frameworks, or technologies while maintaining stability. This skill covers version upgrade strategies, framework migrations, deprecation handling, incremental migration patterns, feature flags, and rollback strategies.

## Instructions

### 1. Version Upgrade Strategies

#### Dependency Audit and Planning

```python
# dependency_audit.py
import subprocess
import json
from dataclasses import dataclass
from typing import List, Dict, Optional
from enum import Enum

class RiskLevel(Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"

@dataclass
class DependencyChange:
    name: str
    current_version: str
    target_version: str
    risk_level: RiskLevel
    breaking_changes: List[str]
    migration_notes: str

class DependencyAuditor:
    def __init__(self, package_manager: str = "npm"):
        self.package_manager = package_manager

    def get_outdated(self) -> Dict[str, dict]:
        """Get list of outdated dependencies."""
        if self.package_manager == "npm":
            result = subprocess.run(
                ["npm", "outdated", "--json"],
                capture_output=True,
                text=True
            )
            return json.loads(result.stdout) if result.stdout else {}
        elif self.package_manager == "pip":
            result = subprocess.run(
                ["pip", "list", "--outdated", "--format=json"],
                capture_output=True,
                text=True
            )
            return {
                pkg["name"]: {
                    "current": pkg["version"],
                    "wanted": pkg["latest_version"]
                }
                for pkg in json.loads(result.stdout)
            }

    def assess_risk(self, change: DependencyChange) -> RiskLevel:
        """Assess risk level of dependency upgrade."""
        current_parts = change.current_version.split(".")
        target_parts = change.target_version.split(".")

        if current_parts[0] != target_parts[0]:
            return RiskLevel.HIGH  # Major version change
        elif current_parts[1] != target_parts[1]:
            return RiskLevel.MEDIUM  # Minor version change
        else:
            return RiskLevel.LOW  # Patch version change

def create_migration_plan(dependencies: List[DependencyChange]) -> str:
    """Generate a migration plan document."""
    plan = ["# Dependency Migration Plan\n"]

    # Group by risk level
    for risk in [RiskLevel.LOW, RiskLevel.MEDIUM, RiskLevel.HIGH, RiskLevel.CRITICAL]:
        deps = [d for d in dependencies if d.risk_level == risk]
        if deps:
            plan.append(f"\n## {risk.value.title()} Risk Updates\n")
            for dep in deps:
                plan.append(f"### {dep.name}: {dep.current_version} -> {dep.target_version}\n")
                if dep.breaking_changes:
                    plan.append("**Breaking Changes:**\n")
                    for change in dep.breaking_changes:
                        plan.append(f"- {change}\n")
                plan.append(f"**Notes:** {dep.migration_notes}\n")

    return "".join(plan)
```

#### Staged Upgrade Process

```python
# staged_upgrade.py
from dataclasses import dataclass
from typing import List, Callable
import subprocess

@dataclass
class UpgradeStage:
    name: str
    description: str
    dependencies: List[str]
    pre_checks: List[Callable[[], bool]]
    post_checks: List[Callable[[], bool]]
    rollback_steps: List[str]

class StagedUpgrader:
    def __init__(self):
        self.stages: List[UpgradeStage] = []
        self.completed_stages: List[str] = []

    def add_stage(self, stage: UpgradeStage):
        self.stages.append(stage)

    def execute(self, dry_run: bool = False) -> bool:
        for stage in self.stages:
            print(f"Executing stage: {stage.name}")

            # Run pre-checks
            for check in stage.pre_checks:
                if not check():
                    print(f"Pre-check failed for stage: {stage.name}")
                    return False

            if not dry_run:
                # Execute upgrade
                for dep in stage.dependencies:
                    self._upgrade_dependency(dep)

                # Run post-checks
                for check in stage.post_checks:
                    if not check():
                        print(f"Post-check failed for stage: {stage.name}")
                        self._rollback(stage)
                        return False

            self.completed_stages.append(stage.name)
            print(f"Completed stage: {stage.name}")

        return True

    def _upgrade_dependency(self, dep: str):
        subprocess.run(["npm", "install", dep], check=True)

    def _rollback(self, failed_stage: UpgradeStage):
        print(f"Rolling back stage: {failed_stage.name}")
        for step in failed_stage.rollback_steps:
            subprocess.run(step, shell=True)

# Usage example
def check_tests_pass() -> bool:
    result = subprocess.run(["npm", "test"], capture_output=True)
    return result.returncode == 0

def check_build_succeeds() -> bool:
    result = subprocess.run(["npm", "run", "build"], capture_output=True)
    return result.returncode == 0

upgrader = StagedUpgrader()
upgrader.add_stage(UpgradeStage(
    name="Core Dependencies",
    description="Upgrade React and related packages",
    dependencies=["react@18", "react-dom@18"],
    pre_checks=[check_tests_pass],
    post_checks=[check_tests_pass, check_build_succeeds],
    rollback_steps=["git checkout package.json", "npm install"]
))
```

### 2. Framework Migrations

#### Adapter Pattern for Gradual Migration

```typescript
// Migrating from Express to Fastify example

// adapter.ts - Common interface
interface RequestAdapter {
  params: Record<string, string>;
  query: Record<string, string>;
  body: unknown;
  headers: Record<string, string>;
}

interface ResponseAdapter {
  status(code: number): ResponseAdapter;
  json(data: unknown): void;
  send(data: string): void;
}

type RouteHandler = (
  req: RequestAdapter,
  res: ResponseAdapter,
) => Promise<void>;

// express-adapter.ts
import { Request, Response } from "express";

function adaptExpressRequest(req: Request): RequestAdapter {
  return {
    params: req.params,
    query: req.query as Record<string, string>,
    body: req.body,
    headers: req.headers as Record<string, string>,
  };
}

function adaptExpressResponse(res: Response): ResponseAdapter {
  return {
    status: (code: number) => {
      res.status(code);
      return adaptExpressResponse(res);
    },
    json: (data: unknown) => res.json(data),
    send: (data: string) => res.send(data),
  };
}

export function wrapForExpress(handler: RouteHandler) {
  return async (req: Request, res: Response) => {
    await handler(adaptExpressRequest(req), adaptExpressResponse(res));
  };
}

// fastify-adapter.ts
import { FastifyRequest, FastifyReply } from "fastify";

function adaptFastifyRequest(req: FastifyRequest): RequestAdapter {
  return {
    params: req.params as Record<string, string>,
    query: req.query as Record<string, string>,
    body: req.body,
    headers: req.headers as Record<string, string>,
  };
}

function adaptFastifyResponse(reply: FastifyReply): ResponseAdapter {
  return {
    status: (code: number) => {
      reply.status(code);
      return adaptFastifyResponse(reply);
    },
    json: (data: unknown) => reply.send(data),
    send: (data: string) => reply.send(data),
  };
}

export function wrapForFastify(handler: RouteHandler) {
  return async (req: FastifyRequest, reply: FastifyReply) => {
    await handler(adaptFastifyRequest(req), adaptFastifyResponse(reply));
  };
}

// handlers/users.ts - Framework-agnostic handlers
export const getUser: RouteHandler = async (req, res) => {
  const { id } = req.params;
  const user = await userService.findById(id);

  if (!user) {
    return res.status(404).json({ error: "User not found" });
  }

  res.json(user);
};

// routes-express.ts (existing)
import express from "express";
import { wrapForExpress } from "./express-adapter";
import { getUser } from "./handlers/users";

const router = express.Router();
router.get("/users/:id", wrapForExpress(getUser));

// routes-fastify.ts (new)
import fastify from "fastify";
import { wrapForFastify } from "./fastify-adapter";
import { getUser } from "./handlers/users";

const app = fastify();
app.get("/users/:id", wrapForFastify(getUser));
```

#### Strangler Fig Pattern

```python
# strangler_fig.py
from fastapi import FastAPI, Request, Response
from httpx import AsyncClient
from typing import Set

class StranglerProxy:
    """Route requests between legacy and new system."""

    def __init__(
        self,
        legacy_url: str,
        migrated_routes: Set[str] = None
    ):
        self.legacy_url = legacy_url
        self.migrated_routes = migrated_routes or set()
        self.client = AsyncClient()

    def is_migrated(self, path: str) -> bool:
        """Check if route has been migrated to new system."""
        for route in self.migrated_routes:
            if path.startswith(route):
                return True
        return False

    async def proxy_to_legacy(self, request: Request) -> Response:
        """Forward request to legacy system."""
        url = f"{self.legacy_url}{request.url.path}"

        response = await self.client.request(
            method=request.method,
            url=url,
            headers=dict(request.headers),
            content=await request.body(),
            params=request.query_params,
        )

        return Response(
            content=response.content,
            status_code=response.status_code,
            headers=dict(response.headers),
        )

# Usage
app = FastAPI()
strangler = StranglerProxy(
    legacy_url="http://legacy-service:8080",
    migrated_routes={"/api/v2/users", "/api/v2/products"}
)

@app.api_route("/{path:path}", methods=["GET", "POST", "PUT", "DELETE"])
async def route_request(request: Request, path: str):
    if strangler.is_migrated(f"/{path}"):
        # Handle in new system (FastAPI routes take precedence)
        raise HTTPException(status_code=404)
    else:
        # Proxy to legacy
        return await strangler.proxy_to_legacy(request)

# Migrated endpoint in new system
@app.get("/api/v2/users/{user_id}")
async def get_user(user_id: str):
    # New implementation
    return await user_service.get(user_id)
```

### 3. Deprecation Handling

```python
# deprecation.py
import warnings
import functools
from typing import Callable, Optional
from datetime import date

def deprecated(
    reason: str,
    replacement: Optional[str] = None,
    removal_version: Optional[str] = None,
    removal_date: Optional[date] = None
):
    """Mark a function as deprecated with migration guidance."""

    def decorator(func: Callable) -> Callable:
        message = f"{func.__name__} is deprecated: {reason}"

        if replacement:
            message += f" Use {replacement} instead."

        if removal_version:
            message += f" Will be removed in version {removal_version}."
        elif removal_date:
            message += f" Will be removed after {removal_date}."

        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            warnings.warn(message, DeprecationWarning, stacklevel=2)
            return func(*args, **kwargs)

        # Add deprecation info to docstring
        wrapper.__doc__ = f"DEPRECATED: {message}\n\n{func.__doc__ or ''}"
        wrapper._deprecated = True
        wrapper._deprecation_info = {
            "reason": reason,
            "replacement": replacement,
            "removal_version": removal_version,
            "removal_date": removal_date,
        }

        return wrapper

    return decorator

# Usage
@deprecated(
    reason="Legacy authentication method",
    replacement="authenticate_v2()",
    removal_version="3.0.0"
)
def authenticate(username: str, password: str) -> bool:
    """Old authentication method."""
    return legacy_auth(username, password)

def authenticate_v2(credentials: Credentials) -> AuthResult:
    """New authentication method with improved security."""
    return modern_auth(credentials)

# Deprecation tracking
class DeprecationTracker:
    """Track deprecated code usage in production."""

    def __init__(self):
        self.usage_counts: Dict[str, int] = {}

    def track(self, func_name: str, caller_info: str):
        key = f"{func_name}:{caller_info}"
        self.usage_counts[key] = self.usage_counts.get(key, 0) + 1

    def get_report(self) -> Dict[str, any]:
        return {
            "total_deprecated_calls": sum(self.usage_counts.values()),
            "by_function": self.usage_counts,
        }

tracker = DeprecationTracker()

def tracked_deprecated(tracker: DeprecationTracker):
    """Decorator that tracks deprecated function usage."""

    def decorator(func: Callable) -> Callable:
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            import traceback
            caller = traceback.extract_stack()[-2]
            tracker.track(func.__name__, f"{caller.filename}:{caller.lineno}")
            return func(*args, **kwargs)
        return wrapper

    return decorator
```

### 4. Incremental Migration Patterns

```python
# incremental_migration.py
from typing import TypeVar, Generic, Callable
from dataclasses import dataclass
from enum import Enum
import random

T = TypeVar('T')

class MigrationPhase(Enum):
    LEGACY_ONLY = "legacy_only"
    SHADOW_MODE = "shadow_mode"       # Run both, compare results
    CANARY = "canary"                 # Small % to new
    GRADUAL_ROLLOUT = "gradual"       # Increasing % to new
    NEW_ONLY = "new_only"

@dataclass
class MigrationConfig:
    phase: MigrationPhase
    new_traffic_percentage: float = 0.0
    shadow_mode_enabled: bool = False
    comparison_enabled: bool = False

class IncrementalMigrator(Generic[T]):
    """Manage incremental migration between implementations."""

    def __init__(
        self,
        legacy_impl: Callable[..., T],
        new_impl: Callable[..., T],
        config: MigrationConfig,
        comparator: Callable[[T, T], bool] = None
    ):
        self.legacy = legacy_impl
        self.new = new_impl
        self.config = config
        self.comparator = comparator or (lambda a, b: a == b)
        self.metrics = MigrationMetrics()

    async def execute(self, *args, **kwargs) -> T:
        phase = self.config.phase

        if phase == MigrationPhase.LEGACY_ONLY:
            return await self.legacy(*args, **kwargs)

        elif phase == MigrationPhase.SHADOW_MODE:
            return await self._execute_shadow(*args, **kwargs)

        elif phase in [MigrationPhase.CANARY, MigrationPhase.GRADUAL_ROLLOUT]:
            return await self._execute_percentage(*args, **kwargs)

        elif phase == MigrationPhase.NEW_ONLY:
            return await self.new(*args, **kwargs)

    async def _execute_shadow(self, *args, **kwargs) -> T:
        """Run both implementations, return legacy result."""
        legacy_result = await self.legacy(*args, **kwargs)

        try:
            new_result = await self.new(*args, **kwargs)

            # Compare results
            if self.config.comparison_enabled:
                matches = self.comparator(legacy_result, new_result)
                self.metrics.record_comparison(matches)

                if not matches:
                    self.metrics.record_mismatch(
                        args, kwargs, legacy_result, new_result
                    )
        except Exception as e:
            self.metrics.record_new_impl_error(e)

        return legacy_result

    async def _execute_percentage(self, *args, **kwargs) -> T:
        """Route traffic based on percentage."""
        use_new = random.random() < self.config.new_traffic_percentage

        if use_new:
            try:
                result = await self.new(*args, **kwargs)
                self.metrics.record_new_success()
                return result
            except Exception as e:
                self.metrics.record_new_impl_error(e)
                # Fallback to legacy
                return await self.legacy(*args, **kwargs)
        else:
            return await self.legacy(*args, **kwargs)

@dataclass
class MigrationMetrics:
    comparisons: int = 0
    matches: int = 0
    mismatches: int = 0
    new_impl_errors: int = 0
    new_impl_successes: int = 0

    def record_comparison(self, matched: bool):
        self.comparisons += 1
        if matched:
            self.matches += 1
        else:
            self.mismatches += 1

    def record_mismatch(self, args, kwargs, legacy, new):
        # Log for debugging
        print(f"Mismatch: legacy={legacy}, new={new}")

    def record_new_impl_error(self, error: Exception):
        self.new_impl_errors += 1

    def record_new_success(self):
        self.new_impl_successes += 1

    @property
    def match_rate(self) -> float:
        if self.comparisons == 0:
            return 0.0
        return self.matches / self.comparisons

# Usage
migrator = IncrementalMigrator(
    legacy_impl=old_search_service.search,
    new_impl=new_search_service.search,
    config=MigrationConfig(
        phase=MigrationPhase.SHADOW_MODE,
        comparison_enabled=True
    ),
    comparator=lambda a, b: set(r.id for r in a) == set(r.id for r in b)
)

results = await migrator.execute(query="test")
```

### 5. Feature Flags for Migrations

```python
# feature_flags.py
from typing import Dict, Any, Optional, Callable
from dataclasses import dataclass
from enum import Enum
import hashlib

class FlagState(Enum):
    OFF = "off"
    ON = "on"
    PERCENTAGE = "percentage"
    USER_SEGMENT = "user_segment"

@dataclass
class FeatureFlag:
    name: str
    state: FlagState
    percentage: float = 0.0
    segments: list = None
    default: bool = False

class FeatureFlagService:
    def __init__(self):
        self.flags: Dict[str, FeatureFlag] = {}
        self._overrides: Dict[str, Dict[str, bool]] = {}

    def register(self, flag: FeatureFlag):
        self.flags[flag.name] = flag

    def is_enabled(
        self,
        flag_name: str,
        user_id: Optional[str] = None,
        context: Dict[str, Any] = None
    ) -> bool:
        flag = self.flags.get(flag_name)
        if not flag:
            return False

        # Check user override
        if user_id and user_id in self._overrides.get(flag_name, {}):
            return self._overrides[flag_name][user_id]

        if flag.state == FlagState.OFF:
            return False
        elif flag.state == FlagState.ON:
            return True
        elif flag.state == FlagState.PERCENTAGE:
            return self._check_percentage(flag_name, user_id, flag.percentage)
        elif flag.state == FlagState.USER_SEGMENT:
            return self._check_segment(user_id, flag.segments, context)

        return flag.default

    def _check_percentage(
        self,
        flag_name: str,
        user_id: Optional[str],
        percentage: float
    ) -> bool:
        """Deterministic percentage check based on user ID."""
        if not user_id:
            return False

        hash_input = f"{flag_name}:{user_id}"
        hash_value = int(hashlib.md5(hash_input.encode()).hexdigest(), 16)
        return (hash_value % 100) < (percentage * 100)

    def _check_segment(
        self,
        user_id: Optional[str],
        segments: list,
        context: Dict[str, Any]
    ) -> bool:
        """Check if user belongs to enabled segment."""
        if not context:
            return False

        user_segment = context.get("segment")
        return user_segment in (segments or [])

    def set_override(self, flag_name: str, user_id: str, enabled: bool):
        """Set per-user override for testing."""
        if flag_name not in self._overrides:
            self._overrides[flag_name] = {}
        self._overrides[flag_name][user_id] = enabled

    def update_percentage(self, flag_name: str, percentage: float):
        """Update rollout percentage."""
        if flag_name in self.flags:
            self.flags[flag_name].percentage = percentage

# Migration-specific flags
flags = FeatureFlagService()

flags.register(FeatureFlag(
    name="use_new_search_api",
    state=FlagState.PERCENTAGE,
    percentage=0.1  # 10% rollout
))

flags.register(FeatureFlag(
    name="use_new_payment_processor",
    state=FlagState.USER_SEGMENT,
    segments=["beta_testers", "employees"]
))

# Usage in code
async def search(query: str, user_id: str):
    if flags.is_enabled("use_new_search_api", user_id):
        return await new_search_service.search(query)
    else:
        return await legacy_search_service.search(query)
```

### 6. Rollback Strategies

```python
# rollback.py
from dataclasses import dataclass
from typing import List, Callable, Optional
from datetime import datetime
import subprocess
import json

@dataclass
class RollbackPoint:
    id: str
    timestamp: datetime
    description: str
    git_commit: str
    db_migration_version: str
    config_snapshot: dict
    rollback_commands: List[str]

class RollbackManager:
    def __init__(self):
        self.rollback_points: List[RollbackPoint] = []
        self.current_point: Optional[RollbackPoint] = None

    def create_rollback_point(
        self,
        description: str,
        rollback_commands: List[str] = None
    ) -> RollbackPoint:
        """Create a rollback point before migration."""
        point = RollbackPoint(
            id=f"rbp_{datetime.now().strftime('%Y%m%d_%H%M%S')}",
            timestamp=datetime.now(),
            description=description,
            git_commit=self._get_current_commit(),
            db_migration_version=self._get_db_version(),
            config_snapshot=self._capture_config(),
            rollback_commands=rollback_commands or []
        )

        self.rollback_points.append(point)
        self._save_point(point)
        return point

    def rollback_to(self, point_id: str) -> bool:
        """Execute rollback to specified point."""
        point = self._find_point(point_id)
        if not point:
            raise ValueError(f"Rollback point {point_id} not found")

        print(f"Rolling back to: {point.description}")

        try:
            # 1. Rollback code
            self._rollback_code(point.git_commit)

            # 2. Rollback database
            self._rollback_database(point.db_migration_version)

            # 3. Restore config
            self._restore_config(point.config_snapshot)

            # 4. Execute custom rollback commands
            for cmd in point.rollback_commands:
                subprocess.run(cmd, shell=True, check=True)

            self.current_point = point
            return True

        except Exception as e:
            print(f"Rollback failed: {e}")
            return False

    def _get_current_commit(self) -> str:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True,
            text=True
        )
        return result.stdout.strip()

    def _get_db_version(self) -> str:
        # Implementation depends on migration tool
        return "migration_version"

    def _capture_config(self) -> dict:
        # Capture current configuration
        return {}

    def _rollback_code(self, commit: str):
        subprocess.run(["git", "checkout", commit], check=True)

    def _rollback_database(self, version: str):
        # Example for Alembic
        subprocess.run(["alembic", "downgrade", version], check=True)

    def _restore_config(self, config: dict):
        # Restore configuration
        pass

    def _save_point(self, point: RollbackPoint):
        # Persist rollback point
        with open(f"rollback_points/{point.id}.json", "w") as f:
            json.dump({
                "id": point.id,
                "timestamp": point.timestamp.isoformat(),
                "description": point.description,
                "git_commit": point.git_commit,
                "db_migration_version": point.db_migration_version,
                "rollback_commands": point.rollback_commands
            }, f)

    def _find_point(self, point_id: str) -> Optional[RollbackPoint]:
        return next(
            (p for p in self.rollback_points if p.id == point_id),
            None
        )

# Automated rollback trigger
class AutoRollback:
    def __init__(
        self,
        rollback_manager: RollbackManager,
        health_check: Callable[[], bool],
        error_threshold: int = 10,
        check_interval: int = 60
    ):
        self.manager = rollback_manager
        self.health_check = health_check
        self.error_threshold = error_threshold
        self.check_interval = check_interval
        self.error_count = 0

    async def monitor(self, rollback_point_id: str):
        """Monitor health and auto-rollback if needed."""
        import asyncio

        while True:
            try:
                if not self.health_check():
                    self.error_count += 1
                    print(f"Health check failed ({self.error_count}/{self.error_threshold})")

                    if self.error_count >= self.error_threshold:
                        print("Error threshold reached, initiating rollback")
                        self.manager.rollback_to(rollback_point_id)
                        break
                else:
                    self.error_count = 0

                await asyncio.sleep(self.check_interval)

            except Exception as e:
                print(f"Monitor error: {e}")
                self.error_count += 1
```

## Best Practices

1. **Plan Thoroughly**: Document migration steps, dependencies, and rollback procedures before starting.

2. **Migrate Incrementally**: Use strangler fig, feature flags, and percentage rollouts to reduce risk.

3. **Shadow Test**: Run new and old systems in parallel to validate behavior before switching.

4. **Monitor Closely**: Track error rates, latency, and business metrics during migration.

5. **Maintain Backward Compatibility**: Keep APIs compatible during transition periods.

6. **Automate Testing**: Comprehensive test suites catch regressions early.

7. **Document Deprecations**: Clear deprecation notices with migration guides help consumers.

8. **Always Have Rollback**: Never deploy a migration without a tested rollback plan.

## Examples

### Complete Migration Workflow

```python
class MigrationWorkflow:
    def __init__(self):
        self.rollback_manager = RollbackManager()
        self.feature_flags = FeatureFlagService()
        self.metrics = MigrationMetrics()

    async def execute_migration(
        self,
        name: str,
        legacy_impl: Callable,
        new_impl: Callable
    ):
        # 1. Create rollback point
        rollback_point = self.rollback_manager.create_rollback_point(
            description=f"Before {name} migration"
        )

        # 2. Register feature flag
        self.feature_flags.register(FeatureFlag(
            name=f"migration_{name}",
            state=FlagState.PERCENTAGE,
            percentage=0.0
        ))

        # 3. Shadow mode testing
        print("Phase 1: Shadow mode testing")
        migrator = IncrementalMigrator(
            legacy_impl=legacy_impl,
            new_impl=new_impl,
            config=MigrationConfig(
                phase=MigrationPhase.SHADOW_MODE,
                comparison_enabled=True
            )
        )

        # Run shadow tests...
        if migrator.metrics.match_rate < 0.99:
            print("Shadow mode match rate too low, aborting")
            return False

        # 4. Canary deployment
        print("Phase 2: Canary deployment (1%)")
        self.feature_flags.update_percentage(f"migration_{name}", 0.01)

        # Monitor for issues...

        # 5. Gradual rollout
        for percentage in [0.05, 0.1, 0.25, 0.5, 0.75, 1.0]:
            print(f"Phase 3: Rolling out to {percentage*100}%")
            self.feature_flags.update_percentage(f"migration_{name}", percentage)
            # Monitor and validate...

        # 6. Complete migration
        print("Migration complete")
        return True
```
