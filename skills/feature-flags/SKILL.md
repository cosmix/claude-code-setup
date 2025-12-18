---
name: feature-flags
description: Feature flag patterns for controlled rollouts, A/B testing, and kill switches. Use when implementing feature toggles, gradual rollouts, canary releases, percentage-based features, user targeting, or emergency kill switches.
---

# Feature Flags

## Overview

Feature flags (also called feature toggles) enable runtime control over feature availability without code deployments. They support gradual rollouts, A/B testing, user targeting, and emergency kill switches. This skill covers implementation patterns, best practices, and integration with popular tools.

## Key Concepts

### Flag Types

**Boolean Flags** - Simple on/off toggles:

```typescript
interface BooleanFlag {
  key: string;
  enabled: boolean;
  description: string;
}

// Usage
if (featureFlags.isEnabled('new-checkout-flow')) {
  return <NewCheckoutFlow />;
}
return <LegacyCheckout />;
```

**Percentage Rollout Flags** - Gradual exposure:

```typescript
interface PercentageFlag {
  key: string;
  percentage: number; // 0-100
  salt: string; // For consistent hashing
}

function isEnabledForUser(flag: PercentageFlag, userId: string): boolean {
  // Consistent hashing ensures same user always gets same result
  const hash = createHash("md5").update(`${flag.salt}:${userId}`).digest("hex");
  const bucket = parseInt(hash.substring(0, 8), 16) % 100;
  return bucket < flag.percentage;
}
```

**User-Targeted Flags** - Specific user segments:

```typescript
interface TargetedFlag {
  key: string;
  defaultValue: boolean;
  rules: TargetingRule[];
}

interface TargetingRule {
  attribute: string; // 'userId', 'email', 'country', 'plan'
  operator: "in" | "notIn" | "equals" | "contains" | "startsWith" | "matches";
  values: string[];
  value: boolean;
}

// Example: Enable for beta users and premium plans
const flag: TargetedFlag = {
  key: "advanced-analytics",
  defaultValue: false,
  rules: [
    {
      attribute: "email",
      operator: "in",
      values: ["beta@example.com"],
      value: true,
    },
    {
      attribute: "plan",
      operator: "in",
      values: ["premium", "enterprise"],
      value: true,
    },
    { attribute: "country", operator: "in", values: ["US", "CA"], value: true },
  ],
};
```

**Multivariate Flags** - Multiple variants for A/B testing:

```typescript
interface MultivariateFlag<T> {
  key: string;
  variants: Variant<T>[];
  defaultVariant: string;
}

interface Variant<T> {
  name: string;
  value: T;
  weight: number; // Percentage allocation
}

// Example: Button color A/B test
const buttonColorFlag: MultivariateFlag<string> = {
  key: "checkout-button-color",
  defaultVariant: "control",
  variants: [
    { name: "control", value: "#007bff", weight: 34 },
    { name: "green", value: "#28a745", weight: 33 },
    { name: "orange", value: "#fd7e14", weight: 33 },
  ],
};
```

### Custom Implementation

```typescript
import { Redis } from "ioredis";
import { createHash } from "crypto";

interface FeatureFlag {
  key: string;
  type: "boolean" | "percentage" | "targeted" | "multivariate";
  enabled: boolean;
  percentage?: number;
  rules?: TargetingRule[];
  variants?: Variant<unknown>[];
  salt: string;
  description: string;
  createdAt: Date;
  updatedAt: Date;
}

interface EvaluationContext {
  userId?: string;
  email?: string;
  country?: string;
  plan?: string;
  [key: string]: string | number | boolean | undefined;
}

class FeatureFlagService {
  private redis: Redis;
  private cache: Map<string, { flag: FeatureFlag; expiresAt: number }> =
    new Map();
  private cacheTTL = 60000; // 1 minute

  constructor(redis: Redis) {
    this.redis = redis;
  }

  async getFlag(key: string): Promise<FeatureFlag | null> {
    // Check local cache
    const cached = this.cache.get(key);
    if (cached && cached.expiresAt > Date.now()) {
      return cached.flag;
    }

    // Fetch from Redis
    const data = await this.redis.get(`flag:${key}`);
    if (!data) return null;

    const flag: FeatureFlag = JSON.parse(data);
    this.cache.set(key, { flag, expiresAt: Date.now() + this.cacheTTL });
    return flag;
  }

  async evaluate(
    key: string,
    context: EvaluationContext = {},
  ): Promise<boolean> {
    const flag = await this.getFlag(key);
    if (!flag) return false;
    if (!flag.enabled) return false;

    switch (flag.type) {
      case "boolean":
        return true;

      case "percentage":
        return this.evaluatePercentage(flag, context.userId || "anonymous");

      case "targeted":
        return this.evaluateTargeting(flag, context);

      default:
        return false;
    }
  }

  async evaluateVariant<T>(
    key: string,
    context: EvaluationContext = {},
  ): Promise<T | null> {
    const flag = await this.getFlag(key);
    if (!flag || !flag.enabled || !flag.variants) return null;

    const userId = context.userId || "anonymous";
    const hash = createHash("md5")
      .update(`${flag.salt}:${userId}`)
      .digest("hex");
    const bucket = parseInt(hash.substring(0, 8), 16) % 100;

    let cumulative = 0;
    for (const variant of flag.variants) {
      cumulative += variant.weight;
      if (bucket < cumulative) {
        return variant.value as T;
      }
    }

    return (flag.variants[0]?.value as T) ?? null;
  }

  private evaluatePercentage(flag: FeatureFlag, userId: string): boolean {
    const hash = createHash("md5")
      .update(`${flag.salt}:${userId}`)
      .digest("hex");
    const bucket = parseInt(hash.substring(0, 8), 16) % 100;
    return bucket < (flag.percentage ?? 0);
  }

  private evaluateTargeting(
    flag: FeatureFlag,
    context: EvaluationContext,
  ): boolean {
    if (!flag.rules || flag.rules.length === 0) return true;

    for (const rule of flag.rules) {
      const attributeValue = String(context[rule.attribute] ?? "");

      let matches = false;
      switch (rule.operator) {
        case "in":
          matches = rule.values.includes(attributeValue);
          break;
        case "notIn":
          matches = !rule.values.includes(attributeValue);
          break;
        case "equals":
          matches = attributeValue === rule.values[0];
          break;
        case "contains":
          matches = rule.values.some((v) => attributeValue.includes(v));
          break;
        case "startsWith":
          matches = rule.values.some((v) => attributeValue.startsWith(v));
          break;
        case "matches":
          matches = rule.values.some((v) => new RegExp(v).test(attributeValue));
          break;
      }

      if (matches) return rule.value;
    }

    return false;
  }

  // Admin methods
  async setFlag(flag: FeatureFlag): Promise<void> {
    await this.redis.set(`flag:${flag.key}`, JSON.stringify(flag));
    this.cache.delete(flag.key);

    // Publish change for other instances
    await this.redis.publish("flag-updates", JSON.stringify({ key: flag.key }));
  }

  async deleteFlag(key: string): Promise<void> {
    await this.redis.del(`flag:${key}`);
    this.cache.delete(key);
    await this.redis.publish(
      "flag-updates",
      JSON.stringify({ key, deleted: true }),
    );
  }
}
```

### Gradual Rollouts and Canary Releases

```typescript
interface RolloutStrategy {
  type: "linear" | "exponential" | "manual";
  startPercentage: number;
  targetPercentage: number;
  incrementPercentage: number;
  intervalMinutes: number;
  currentPercentage: number;
  startedAt: Date;
  pausedAt?: Date;
}

class RolloutManager {
  private flags: FeatureFlagService;

  async startRollout(
    flagKey: string,
    strategy: RolloutStrategy,
  ): Promise<void> {
    const flag = await this.flags.getFlag(flagKey);
    if (!flag) throw new Error("Flag not found");

    flag.percentage = strategy.startPercentage;
    flag.rolloutStrategy = strategy;
    await this.flags.setFlag(flag);

    // Schedule automatic increments
    if (strategy.type !== "manual") {
      this.scheduleIncrement(flagKey, strategy);
    }
  }

  private async scheduleIncrement(
    flagKey: string,
    strategy: RolloutStrategy,
  ): Promise<void> {
    const incrementJob = async () => {
      const flag = await this.flags.getFlag(flagKey);
      if (!flag || flag.rolloutStrategy?.pausedAt) return;

      const current = flag.percentage ?? 0;
      let newPercentage: number;

      if (strategy.type === "linear") {
        newPercentage = Math.min(
          current + strategy.incrementPercentage,
          strategy.targetPercentage,
        );
      } else {
        // Exponential: double each time
        newPercentage = Math.min(current * 2, strategy.targetPercentage);
      }

      flag.percentage = newPercentage;
      await this.flags.setFlag(flag);

      // Continue if not at target
      if (newPercentage < strategy.targetPercentage) {
        setTimeout(incrementJob, strategy.intervalMinutes * 60 * 1000);
      }
    };

    setTimeout(incrementJob, strategy.intervalMinutes * 60 * 1000);
  }

  async pauseRollout(flagKey: string): Promise<void> {
    const flag = await this.flags.getFlag(flagKey);
    if (flag?.rolloutStrategy) {
      flag.rolloutStrategy.pausedAt = new Date();
      await this.flags.setFlag(flag);
    }
  }

  async rollback(flagKey: string): Promise<void> {
    const flag = await this.flags.getFlag(flagKey);
    if (flag) {
      flag.percentage = 0;
      flag.enabled = false;
      await this.flags.setFlag(flag);
    }
  }
}
```

### A/B Testing Integration

```typescript
interface Experiment {
  id: string;
  flagKey: string;
  name: string;
  hypothesis: string;
  metrics: string[]; // Metrics to track
  variants: ExperimentVariant[];
  status: "draft" | "running" | "paused" | "completed";
  startDate?: Date;
  endDate?: Date;
  sampleSize: number;
  confidenceLevel: number; // e.g., 0.95
}

interface ExperimentVariant {
  name: string;
  weight: number;
  conversions: number;
  impressions: number;
}

class ABTestingService {
  async trackExposure(
    experimentId: string,
    variantName: string,
    userId: string,
  ): Promise<void> {
    // Record that user was exposed to variant
    await this.analytics.track({
      event: "experiment_exposure",
      userId,
      properties: {
        experimentId,
        variant: variantName,
        timestamp: new Date(),
      },
    });

    // Increment impression count
    await this.redis.hincrby(
      `experiment:${experimentId}:${variantName}`,
      "impressions",
      1,
    );
  }

  async trackConversion(
    experimentId: string,
    userId: string,
    metric: string,
  ): Promise<void> {
    // Get user's assigned variant
    const variant = await this.redis.hget(
      `experiment:${experimentId}:assignments`,
      userId,
    );
    if (!variant) return;

    await this.analytics.track({
      event: "experiment_conversion",
      userId,
      properties: {
        experimentId,
        variant,
        metric,
        timestamp: new Date(),
      },
    });

    await this.redis.hincrby(
      `experiment:${experimentId}:${variant}`,
      "conversions",
      1,
    );
  }

  async getResults(experimentId: string): Promise<ExperimentResults> {
    const experiment = await this.getExperiment(experimentId);

    const results = await Promise.all(
      experiment.variants.map(async (variant) => {
        const data = await this.redis.hgetall(
          `experiment:${experimentId}:${variant.name}`,
        );
        return {
          name: variant.name,
          impressions: parseInt(data.impressions || "0"),
          conversions: parseInt(data.conversions || "0"),
          conversionRate:
            parseInt(data.conversions || "0") /
            parseInt(data.impressions || "1"),
        };
      }),
    );

    // Calculate statistical significance
    const control = results.find((r) => r.name === "control");
    const treatments = results.filter((r) => r.name !== "control");

    return {
      experimentId,
      results,
      winners: treatments.filter((t) =>
        this.isStatisticallySignificant(
          control!,
          t,
          experiment.confidenceLevel,
        ),
      ),
    };
  }

  private isStatisticallySignificant(
    control: VariantResult,
    treatment: VariantResult,
    confidenceLevel: number,
  ): boolean {
    // Z-test for proportions
    const p1 = control.conversionRate;
    const p2 = treatment.conversionRate;
    const n1 = control.impressions;
    const n2 = treatment.impressions;

    const pooledP = (p1 * n1 + p2 * n2) / (n1 + n2);
    const se = Math.sqrt(pooledP * (1 - pooledP) * (1 / n1 + 1 / n2));
    const z = (p2 - p1) / se;

    // Z-score for 95% confidence is 1.96
    const zThreshold = confidenceLevel === 0.95 ? 1.96 : 2.58; // 99%
    return Math.abs(z) > zThreshold;
  }
}
```

### Kill Switches

```typescript
interface KillSwitch {
  key: string;
  description: string;
  affectedServices: string[];
  activatedAt?: Date;
  activatedBy?: string;
  reason?: string;
  autoRecoveryMinutes?: number;
}

class KillSwitchService {
  private redis: Redis;
  private alerting: AlertingService;

  async activate(
    key: string,
    reason: string,
    activatedBy: string,
  ): Promise<void> {
    const killSwitch = await this.getKillSwitch(key);
    if (!killSwitch) throw new Error("Kill switch not found");

    killSwitch.activatedAt = new Date();
    killSwitch.activatedBy = activatedBy;
    killSwitch.reason = reason;

    await this.redis.set(`killswitch:${key}`, JSON.stringify(killSwitch));

    // Broadcast to all instances immediately
    await this.redis.publish(
      "killswitch-activated",
      JSON.stringify(killSwitch),
    );

    // Alert on-call
    await this.alerting.sendCritical({
      title: `Kill Switch Activated: ${key}`,
      message: `Reason: ${reason}\nActivated by: ${activatedBy}\nAffected: ${killSwitch.affectedServices.join(", ")}`,
    });

    // Schedule auto-recovery if configured
    if (killSwitch.autoRecoveryMinutes) {
      setTimeout(
        () => this.deactivate(key, "Auto-recovery"),
        killSwitch.autoRecoveryMinutes * 60 * 1000,
      );
    }
  }

  async deactivate(key: string, reason: string): Promise<void> {
    const killSwitch = await this.getKillSwitch(key);
    if (!killSwitch) return;

    killSwitch.activatedAt = undefined;
    killSwitch.activatedBy = undefined;
    killSwitch.reason = undefined;

    await this.redis.set(`killswitch:${key}`, JSON.stringify(killSwitch));
    await this.redis.publish(
      "killswitch-deactivated",
      JSON.stringify({ key, reason }),
    );

    await this.alerting.sendInfo({
      title: `Kill Switch Deactivated: ${key}`,
      message: `Reason: ${reason}`,
    });
  }

  async isActive(key: string): Promise<boolean> {
    const data = await this.redis.get(`killswitch:${key}`);
    if (!data) return false;
    const killSwitch: KillSwitch = JSON.parse(data);
    return !!killSwitch.activatedAt;
  }
}

// Usage in application code
async function processPayment(payment: Payment): Promise<PaymentResult> {
  // Check kill switch first
  if (await killSwitches.isActive("payments-disabled")) {
    throw new ServiceUnavailableError(
      "Payment processing temporarily disabled",
    );
  }

  // Normal processing
  return paymentProcessor.process(payment);
}
```

### Flag Lifecycle Management

```typescript
interface FlagLifecycle {
  key: string;
  status:
    | "planning"
    | "development"
    | "testing"
    | "rollout"
    | "stable"
    | "deprecated"
    | "removed";
  owner: string;
  team: string;
  createdAt: Date;
  plannedRemovalDate?: Date;
  jiraTicket?: string;
  staleAfterDays: number;
}

class FlagLifecycleManager {
  async checkStaleFlags(): Promise<StaleFlag[]> {
    const flags = await this.getAllFlags();
    const now = new Date();

    return flags.filter((flag) => {
      const lifecycle = flag.lifecycle;
      if (!lifecycle) return false;

      const ageInDays =
        (now.getTime() - lifecycle.createdAt.getTime()) / (1000 * 60 * 60 * 24);

      // Flag is stale if:
      // 1. It's older than staleAfterDays and still in development/testing
      // 2. It's past its planned removal date
      // 3. It's been stable for > 30 days (should be permanent or removed)

      if (lifecycle.status === "stable" && ageInDays > 30) return true;
      if (lifecycle.plannedRemovalDate && lifecycle.plannedRemovalDate < now)
        return true;
      if (
        ["development", "testing"].includes(lifecycle.status) &&
        ageInDays > lifecycle.staleAfterDays
      )
        return true;

      return false;
    });
  }

  async generateCleanupReport(): Promise<CleanupReport> {
    const staleFlags = await this.checkStaleFlags();

    return {
      generatedAt: new Date(),
      staleFlags: staleFlags.map((flag) => ({
        key: flag.key,
        status: flag.lifecycle.status,
        owner: flag.lifecycle.owner,
        age: this.calculateAge(flag.lifecycle.createdAt),
        recommendation: this.getRecommendation(flag),
      })),
      summary: {
        total: staleFlags.length,
        byStatus: this.groupBy(staleFlags, (f) => f.lifecycle.status),
        byTeam: this.groupBy(staleFlags, (f) => f.lifecycle.team),
      },
    };
  }

  private getRecommendation(flag: FlagWithLifecycle): string {
    const { status, plannedRemovalDate } = flag.lifecycle;

    if (plannedRemovalDate && plannedRemovalDate < new Date()) {
      return "URGENT: Past planned removal date. Remove flag and clean up code.";
    }
    if (status === "stable") {
      return "Flag is stable. Either make permanent (remove flag, keep feature) or deprecate.";
    }
    if (status === "development" || status === "testing") {
      return "Flag stuck in development. Complete rollout or remove if abandoned.";
    }
    return "Review flag status and update lifecycle.";
  }
}
```

### LaunchDarkly Integration

```typescript
import * as LaunchDarkly from "launchdarkly-node-server-sdk";

const ldClient = LaunchDarkly.init(process.env.LAUNCHDARKLY_SDK_KEY!);

interface LDUser {
  key: string;
  email?: string;
  name?: string;
  custom?: Record<string, string | number | boolean>;
}

async function evaluateFlag(
  flagKey: string,
  user: LDUser,
  defaultValue: boolean,
): Promise<boolean> {
  await ldClient.waitForInitialization();
  return ldClient.variation(flagKey, user, defaultValue);
}

async function evaluateFlagWithReason(
  flagKey: string,
  user: LDUser,
  defaultValue: boolean,
) {
  await ldClient.waitForInitialization();
  const detail = await ldClient.variationDetail(flagKey, user, defaultValue);

  return {
    value: detail.value,
    reason: detail.reason,
    variationIndex: detail.variationIndex,
  };
}

// Track custom events for experiments
function trackConversion(
  user: LDUser,
  eventKey: string,
  data?: Record<string, unknown>,
): void {
  ldClient.track(eventKey, user, data);
}

// React hook for client-side
function useFeatureFlag(
  flagKey: string,
  defaultValue: boolean = false,
): boolean {
  const ldClient = useLDClient();
  const [value, setValue] = useState(defaultValue);

  useEffect(() => {
    if (!ldClient) return;

    setValue(ldClient.variation(flagKey, defaultValue));

    const handler = (newValue: boolean) => setValue(newValue);
    ldClient.on(`change:${flagKey}`, handler);

    return () => ldClient.off(`change:${flagKey}`, handler);
  }, [ldClient, flagKey, defaultValue]);

  return value;
}
```

## Best Practices

1. **Flag Naming Conventions**
   - Use descriptive, consistent names: `feature-checkout-v2`, `experiment-button-color`
   - Include type prefix: `release-*`, `experiment-*`, `ops-*`, `kill-*`
   - Avoid abbreviations and ensure team-wide understanding

2. **Flag Hygiene**
   - Set expiration dates for temporary flags
   - Remove flags after features are fully rolled out
   - Track flag ownership and associated tickets
   - Regular cleanup audits (monthly)

3. **Testing**
   - Test all flag states (on, off, each variant)
   - Include flag states in integration tests
   - Test rollback scenarios

4. **Monitoring**
   - Track flag evaluation counts and latency
   - Alert on unusual patterns (sudden spikes, failures)
   - Log flag decisions for debugging

5. **Documentation**
   - Document what each flag controls
   - Include rollback instructions
   - Link to related PRs and tickets

## Examples

### React Feature Flag Provider

```typescript
import React, { createContext, useContext, useEffect, useState } from 'react';

interface FeatureFlagContextType {
  isEnabled: (key: string) => boolean;
  getVariant: <T>(key: string) => T | null;
  loading: boolean;
}

const FeatureFlagContext = createContext<FeatureFlagContextType | null>(null);

export function FeatureFlagProvider({ children, userId }: { children: React.ReactNode; userId: string }) {
  const [flags, setFlags] = useState<Record<string, unknown>>({});
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadFlags() {
      const response = await fetch(`/api/flags?userId=${userId}`);
      const data = await response.json();
      setFlags(data);
      setLoading(false);
    }
    loadFlags();

    // Subscribe to real-time updates
    const ws = new WebSocket(`wss://api.example.com/flags/stream?userId=${userId}`);
    ws.onmessage = (event) => {
      const update = JSON.parse(event.data);
      setFlags(prev => ({ ...prev, [update.key]: update.value }));
    };

    return () => ws.close();
  }, [userId]);

  const value: FeatureFlagContextType = {
    isEnabled: (key) => Boolean(flags[key]),
    getVariant: (key) => flags[key] as T ?? null,
    loading,
  };

  return (
    <FeatureFlagContext.Provider value={value}>
      {children}
    </FeatureFlagContext.Provider>
  );
}

export function useFeatureFlag(key: string): boolean {
  const context = useContext(FeatureFlagContext);
  if (!context) throw new Error('useFeatureFlag must be used within FeatureFlagProvider');
  return context.isEnabled(key);
}

// Usage
function CheckoutPage() {
  const newCheckout = useFeatureFlag('new-checkout-flow');

  if (newCheckout) {
    return <NewCheckoutFlow />;
  }
  return <LegacyCheckout />;
}
```
