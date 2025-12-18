---
name: performance-testing
description: Performance testing guidance including load testing with k6, locust, and artillery, benchmarking strategies, profiling techniques, metrics analysis, performance budgets, and bottleneck identification. Use when setting up performance tests, analyzing system behavior under load, or optimizing application performance. Trigger keywords: performance testing, load testing, k6, locust, artillery, benchmarking, profiling, latency, throughput, performance budget, bottleneck, stress testing, scalability testing.
---

# Performance Testing

## Overview

Performance testing validates that applications meet speed, scalability, and stability requirements under various load conditions. This skill covers load testing tools, benchmarking strategies, profiling techniques, and systematic approaches to identifying and resolving performance bottlenecks.

## Instructions

### 1. Load Testing with Modern Tools

**k6 (Recommended for API Load Testing):**

```bash
# Installation
brew install k6
# or
npm install -g k6
```

```javascript
// load-test.js
import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";

// Custom metrics
const errorRate = new Rate("errors");
const latencyTrend = new Trend("latency");

export const options = {
  stages: [
    { duration: "2m", target: 100 }, // Ramp up
    { duration: "5m", target: 100 }, // Steady state
    { duration: "2m", target: 200 }, // Spike
    { duration: "5m", target: 200 }, // Sustained spike
    { duration: "2m", target: 0 }, // Ramp down
  ],
  thresholds: {
    http_req_duration: ["p(95)<500", "p(99)<1000"],
    errors: ["rate<0.01"],
    http_req_failed: ["rate<0.01"],
  },
};

export default function () {
  const payload = JSON.stringify({
    username: `user_${__VU}_${__ITER}`,
    action: "test",
  });

  const params = {
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${__ENV.API_TOKEN}`,
    },
  };

  const response = http.post(
    "https://api.example.com/endpoint",
    payload,
    params,
  );

  latencyTrend.add(response.timings.duration);
  errorRate.add(response.status !== 200);

  check(response, {
    "status is 200": (r) => r.status === 200,
    "response time < 500ms": (r) => r.timings.duration < 500,
    "has required fields": (r) => {
      const body = JSON.parse(r.body);
      return body.id && body.status;
    },
  });

  sleep(1);
}

export function handleSummary(data) {
  return {
    "summary.json": JSON.stringify(data),
    stdout: textSummary(data, { indent: " ", enableColors: true }),
  };
}
```

**Run k6 Tests:**

```bash
# Basic run
k6 run load-test.js

# With environment variables
k6 run -e API_TOKEN=xxx load-test.js

# Cloud execution
k6 cloud load-test.js

# Output to InfluxDB for Grafana
k6 run --out influxdb=http://localhost:8086/k6 load-test.js
```

**Locust (Python-based Load Testing):**

```python
# locustfile.py
from locust import HttpUser, task, between
from locust import events
import time

class WebsiteUser(HttpUser):
    wait_time = between(1, 3)

    def on_start(self):
        """Login on start"""
        response = self.client.post("/login", json={
            "username": "testuser",
            "password": "testpass"
        })
        self.token = response.json().get("token")

    @task(3)
    def view_products(self):
        """Most common action"""
        self.client.get("/api/products", headers={
            "Authorization": f"Bearer {self.token}"
        })

    @task(2)
    def view_product_detail(self):
        """View individual product"""
        self.client.get("/api/products/1", headers={
            "Authorization": f"Bearer {self.token}"
        })

    @task(1)
    def add_to_cart(self):
        """Less common action"""
        self.client.post("/api/cart", json={
            "product_id": 1,
            "quantity": 1
        }, headers={
            "Authorization": f"Bearer {self.token}"
        })

class AdminUser(HttpUser):
    wait_time = between(2, 5)
    weight = 1  # 1 admin per 10 regular users

    @task
    def view_dashboard(self):
        self.client.get("/admin/dashboard")

# Custom metrics
@events.request.add_listener
def on_request(request_type, name, response_time, response_length, exception, **kwargs):
    if exception:
        print(f"Request failed: {name} - {exception}")
```

**Run Locust:**

```bash
# Web UI mode
locust -f locustfile.py --host=https://api.example.com

# Headless mode
locust -f locustfile.py --headless -u 100 -r 10 --run-time 5m --host=https://api.example.com

# Distributed mode
locust -f locustfile.py --master
locust -f locustfile.py --worker --master-host=192.168.1.1
```

**Artillery (YAML-based Load Testing):**

```yaml
# artillery-config.yml
config:
  target: "https://api.example.com"
  phases:
    - duration: 60
      arrivalRate: 5
      name: "Warm up"
    - duration: 120
      arrivalRate: 20
      rampTo: 50
      name: "Ramp up"
    - duration: 300
      arrivalRate: 50
      name: "Sustained load"
  defaults:
    headers:
      Content-Type: "application/json"
  plugins:
    expect: {}
  ensure:
    p95: 500
    maxErrorRate: 1

scenarios:
  - name: "User journey"
    flow:
      - post:
          url: "/auth/login"
          json:
            username: "{{ $randomString() }}"
            password: "password123"
          capture:
            - json: "$.token"
              as: "authToken"
          expect:
            - statusCode: 200
            - hasProperty: "token"

      - get:
          url: "/api/products"
          headers:
            Authorization: "Bearer {{ authToken }}"
          expect:
            - statusCode: 200
            - contentType: "application/json"

      - think: 2

      - post:
          url: "/api/cart"
          headers:
            Authorization: "Bearer {{ authToken }}"
          json:
            productId: "{{ $randomNumber(1, 100) }}"
            quantity: 1
          expect:
            - statusCode: 201
```

**Run Artillery:**

```bash
# Run test
artillery run artillery-config.yml

# Generate report
artillery run artillery-config.yml --output report.json
artillery report report.json --output report.html
```

### 2. Benchmarking Strategies

**Micro-benchmarking (Function Level):**

```typescript
// Node.js with benchmark.js
import Benchmark from "benchmark";

const suite = new Benchmark.Suite();

const data = Array.from({ length: 10000 }, (_, i) => i);

suite
  .add("for loop", function () {
    let sum = 0;
    for (let i = 0; i < data.length; i++) {
      sum += data[i];
    }
    return sum;
  })
  .add("forEach", function () {
    let sum = 0;
    data.forEach((n) => {
      sum += n;
    });
    return sum;
  })
  .add("reduce", function () {
    return data.reduce((sum, n) => sum + n, 0);
  })
  .on("cycle", function (event: Benchmark.Event) {
    console.log(String(event.target));
  })
  .on("complete", function (this: Benchmark.Suite) {
    console.log("Fastest is " + this.filter("fastest").map("name"));
  })
  .run({ async: true });
```

**Database Query Benchmarking:**

```typescript
// benchmark-queries.ts
import { performance } from "perf_hooks";

interface BenchmarkResult {
  query: string;
  avgTime: number;
  minTime: number;
  maxTime: number;
  p95: number;
  iterations: number;
}

async function benchmarkQuery(
  name: string,
  queryFn: () => Promise<any>,
  iterations: number = 100,
): Promise<BenchmarkResult> {
  const times: number[] = [];

  // Warm up
  for (let i = 0; i < 10; i++) {
    await queryFn();
  }

  // Actual benchmark
  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    await queryFn();
    times.push(performance.now() - start);
  }

  times.sort((a, b) => a - b);

  return {
    query: name,
    avgTime: times.reduce((a, b) => a + b) / times.length,
    minTime: times[0],
    maxTime: times[times.length - 1],
    p95: times[Math.floor(times.length * 0.95)],
    iterations,
  };
}

// Usage
const results = await Promise.all([
  benchmarkQuery("findUserById", () => db.users.findById(1)),
  benchmarkQuery("findUserWithJoin", () =>
    db.users.findById(1).include("orders"),
  ),
  benchmarkQuery("complexAggregation", () =>
    db.orders.aggregate([
      /* pipeline */
    ]),
  ),
]);

console.table(results);
```

**HTTP Endpoint Benchmarking:**

```bash
# Using wrk
wrk -t12 -c400 -d30s --latency https://api.example.com/endpoint

# Using autocannon (Node.js)
npx autocannon -c 100 -d 30 -p 10 https://api.example.com/endpoint

# Using hey
hey -n 10000 -c 100 https://api.example.com/endpoint
```

### 3. Profiling Techniques

**Node.js CPU Profiling:**

```typescript
// Enable built-in profiler
// node --prof app.js
// node --prof-process isolate-*.log > processed.txt

// Programmatic profiling
import { Session } from "inspector";
import { writeFileSync } from "fs";

async function profileFunction(fn: () => Promise<any>) {
  const session = new Session();
  session.connect();

  session.post("Profiler.enable");
  session.post("Profiler.start");

  await fn();

  return new Promise<void>((resolve) => {
    session.post("Profiler.stop", (err, { profile }) => {
      writeFileSync("profile.cpuprofile", JSON.stringify(profile));
      session.disconnect();
      resolve();
    });
  });
}

// Usage
await profileFunction(async () => {
  // Code to profile
  await heavyComputation();
});
// Open profile.cpuprofile in Chrome DevTools
```

**Memory Profiling:**

```typescript
// Memory snapshot
import v8 from "v8";
import { writeFileSync } from "fs";

function takeHeapSnapshot(filename: string) {
  const snapshotStream = v8.writeHeapSnapshot(filename);
  console.log(`Heap snapshot written to ${snapshotStream}`);
}

// Track memory usage
function logMemoryUsage(label: string) {
  const usage = process.memoryUsage();
  console.log(`Memory [${label}]:`, {
    heapUsed: `${Math.round(usage.heapUsed / 1024 / 1024)}MB`,
    heapTotal: `${Math.round(usage.heapTotal / 1024 / 1024)}MB`,
    external: `${Math.round(usage.external / 1024 / 1024)}MB`,
    rss: `${Math.round(usage.rss / 1024 / 1024)}MB`,
  });
}

// Detect memory leaks
class MemoryLeakDetector {
  private samples: number[] = [];
  private interval: NodeJS.Timer | null = null;

  start(sampleInterval: number = 1000) {
    this.interval = setInterval(() => {
      this.samples.push(process.memoryUsage().heapUsed);

      if (this.samples.length > 60) {
        const trend = this.calculateTrend();
        if (trend > 0.1) {
          // 10% growth per minute
          console.warn("Potential memory leak detected!");
        }
        this.samples.shift();
      }
    }, sampleInterval);
  }

  private calculateTrend(): number {
    if (this.samples.length < 2) return 0;
    const first = this.samples[0];
    const last = this.samples[this.samples.length - 1];
    return (last - first) / first;
  }

  stop() {
    if (this.interval) clearInterval(this.interval);
  }
}
```

**Database Query Profiling:**

```sql
-- PostgreSQL: Enable query logging
SET log_statement = 'all';
SET log_duration = on;

-- Analyze query plan
EXPLAIN ANALYZE SELECT * FROM users
WHERE created_at > '2024-01-01'
ORDER BY created_at DESC
LIMIT 100;

-- Find slow queries
SELECT
  query,
  calls,
  total_time / 1000 as total_seconds,
  mean_time / 1000 as mean_seconds,
  rows
FROM pg_stat_statements
ORDER BY total_time DESC
LIMIT 20;
```

```typescript
// Application-level query profiling
import { performance } from "perf_hooks";

const queryLogger = {
  queries: [] as Array<{ sql: string; duration: number; timestamp: Date }>,

  log(sql: string, duration: number) {
    this.queries.push({ sql, duration, timestamp: new Date() });
    if (duration > 100) {
      console.warn(`Slow query (${duration}ms): ${sql.substring(0, 100)}...`);
    }
  },

  getSlowQueries(threshold: number = 100) {
    return this.queries.filter((q) => q.duration > threshold);
  },

  getStats() {
    const durations = this.queries.map((q) => q.duration);
    return {
      count: durations.length,
      avg: durations.reduce((a, b) => a + b, 0) / durations.length,
      max: Math.max(...durations),
      p95: durations.sort((a, b) => a - b)[Math.floor(durations.length * 0.95)],
    };
  },
};
```

### 4. Track Key Metrics

**Essential Performance Metrics:**

| Metric        | Description             | Target    | Alert Threshold |
| ------------- | ----------------------- | --------- | --------------- |
| Latency (p50) | Median response time    | <100ms    | >200ms          |
| Latency (p95) | 95th percentile         | <500ms    | >1000ms         |
| Latency (p99) | 99th percentile         | <1000ms   | >2000ms         |
| Throughput    | Requests per second     | >1000 RPS | <500 RPS        |
| Error Rate    | Failed requests %       | <0.1%     | >1%             |
| Saturation    | Resource utilization    | <70%      | >85%            |
| Apdex         | User satisfaction score | >0.9      | <0.7            |

**Implementing Metrics Collection:**

```typescript
// metrics.ts
import { Counter, Histogram, Gauge, Registry } from "prom-client";

const register = new Registry();

// Request metrics
const httpRequestDuration = new Histogram({
  name: "http_request_duration_seconds",
  help: "Duration of HTTP requests in seconds",
  labelNames: ["method", "route", "status_code"],
  buckets: [0.01, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10],
  registers: [register],
});

const httpRequestTotal = new Counter({
  name: "http_requests_total",
  help: "Total number of HTTP requests",
  labelNames: ["method", "route", "status_code"],
  registers: [register],
});

const activeConnections = new Gauge({
  name: "active_connections",
  help: "Number of active connections",
  registers: [register],
});

// Middleware for Express
export function metricsMiddleware(
  req: Request,
  res: Response,
  next: NextFunction,
) {
  const start = process.hrtime.bigint();

  activeConnections.inc();

  res.on("finish", () => {
    const duration = Number(process.hrtime.bigint() - start) / 1e9;
    const route = req.route?.path || req.path;

    httpRequestDuration.observe(
      { method: req.method, route, status_code: res.statusCode },
      duration,
    );

    httpRequestTotal.inc({
      method: req.method,
      route,
      status_code: res.statusCode,
    });

    activeConnections.dec();
  });

  next();
}

// Metrics endpoint
app.get("/metrics", async (req, res) => {
  res.set("Content-Type", register.contentType);
  res.end(await register.metrics());
});
```

**Custom Business Metrics:**

```typescript
// business-metrics.ts
const orderProcessingTime = new Histogram({
  name: "order_processing_duration_seconds",
  help: "Time to process an order",
  labelNames: ["payment_method", "status"],
  buckets: [0.5, 1, 2, 5, 10, 30, 60],
});

const cartValue = new Histogram({
  name: "cart_value_dollars",
  help: "Shopping cart value at checkout",
  buckets: [10, 25, 50, 100, 250, 500, 1000],
});

const concurrentUsers = new Gauge({
  name: "concurrent_authenticated_users",
  help: "Number of currently authenticated users",
});
```

### 5. Define Performance Budgets

**Web Performance Budgets:**

```typescript
// performance-budget.ts
interface PerformanceBudget {
  metric: string;
  budget: number;
  unit: string;
}

const webBudgets: PerformanceBudget[] = [
  // Timing metrics
  { metric: "First Contentful Paint", budget: 1800, unit: "ms" },
  { metric: "Largest Contentful Paint", budget: 2500, unit: "ms" },
  { metric: "Time to Interactive", budget: 3800, unit: "ms" },
  { metric: "Total Blocking Time", budget: 300, unit: "ms" },
  { metric: "Cumulative Layout Shift", budget: 0.1, unit: "" },

  // Resource budgets
  { metric: "JavaScript bundle size", budget: 300, unit: "KB" },
  { metric: "CSS bundle size", budget: 100, unit: "KB" },
  { metric: "Total page weight", budget: 1500, unit: "KB" },
  { metric: "Image weight", budget: 500, unit: "KB" },

  // Request budgets
  { metric: "Total requests", budget: 50, unit: "requests" },
  { metric: "Third-party requests", budget: 10, unit: "requests" },
];
```

**Lighthouse CI Configuration:**

```javascript
// lighthouserc.js
module.exports = {
  ci: {
    collect: {
      url: ["http://localhost:3000/", "http://localhost:3000/products"],
      numberOfRuns: 3,
    },
    assert: {
      assertions: {
        "first-contentful-paint": ["error", { maxNumericValue: 1800 }],
        "largest-contentful-paint": ["error", { maxNumericValue: 2500 }],
        interactive: ["error", { maxNumericValue: 3800 }],
        "total-blocking-time": ["error", { maxNumericValue: 300 }],
        "cumulative-layout-shift": ["error", { maxNumericValue: 0.1 }],
        "resource-summary:script:size": ["error", { maxNumericValue: 300000 }],
        "resource-summary:total:size": ["error", { maxNumericValue: 1500000 }],
      },
    },
    upload: {
      target: "temporary-public-storage",
    },
  },
};
```

**API Performance Budgets:**

```yaml
# api-budgets.yml
endpoints:
  GET /api/products:
    p50_latency_ms: 50
    p95_latency_ms: 200
    p99_latency_ms: 500
    error_rate_percent: 0.1
    throughput_rps: 1000

  POST /api/orders:
    p50_latency_ms: 200
    p95_latency_ms: 800
    p99_latency_ms: 2000
    error_rate_percent: 0.01
    throughput_rps: 100

  GET /api/search:
    p50_latency_ms: 100
    p95_latency_ms: 500
    p99_latency_ms: 1500
    error_rate_percent: 0.5
    throughput_rps: 500
```

### 6. Identify and Resolve Bottlenecks

**Systematic Bottleneck Analysis:**

```typescript
// bottleneck-analyzer.ts
interface BottleneckReport {
  category: "cpu" | "memory" | "io" | "network" | "database";
  severity: "low" | "medium" | "high" | "critical";
  description: string;
  recommendation: string;
  metrics: Record<string, number>;
}

async function analyzeBottlenecks(): Promise<BottleneckReport[]> {
  const reports: BottleneckReport[] = [];

  // CPU analysis
  const cpuUsage = process.cpuUsage();
  if (cpuUsage.user / 1000000 > 80) {
    reports.push({
      category: "cpu",
      severity: "high",
      description: "High CPU utilization detected",
      recommendation: "Profile CPU usage, optimize hot paths, consider caching",
      metrics: { userCpuPercent: cpuUsage.user / 1000000 },
    });
  }

  // Memory analysis
  const memUsage = process.memoryUsage();
  const heapUsedPercent = (memUsage.heapUsed / memUsage.heapTotal) * 100;
  if (heapUsedPercent > 85) {
    reports.push({
      category: "memory",
      severity: "high",
      description: "High memory pressure detected",
      recommendation: "Check for memory leaks, reduce object retention",
      metrics: { heapUsedPercent, heapUsedMB: memUsage.heapUsed / 1024 / 1024 },
    });
  }

  // Event loop lag
  const lagStart = Date.now();
  await new Promise((resolve) => setImmediate(resolve));
  const eventLoopLag = Date.now() - lagStart;
  if (eventLoopLag > 100) {
    reports.push({
      category: "cpu",
      severity: "medium",
      description: "Event loop blocking detected",
      recommendation: "Move CPU-intensive work to worker threads",
      metrics: { eventLoopLagMs: eventLoopLag },
    });
  }

  return reports;
}
```

**Common Bottlenecks and Solutions:**

| Bottleneck       | Symptoms                     | Diagnosis                | Solution                          |
| ---------------- | ---------------------------- | ------------------------ | --------------------------------- |
| N+1 Queries      | Linear latency increase      | Query logging            | Eager loading, batching           |
| Missing Index    | Slow queries on large tables | EXPLAIN ANALYZE          | Add appropriate indexes           |
| Connection Pool  | Timeouts under load          | Pool metrics             | Increase pool size, add queueing  |
| Synchronous I/O  | High event loop lag          | Profiling                | Use async operations              |
| Memory Leak      | Growing heap over time       | Heap snapshots           | Fix object retention              |
| Unoptimized JSON | High CPU on serialization    | CPU profiling            | Stream parsing, schema validation |
| Large Payloads   | High network latency         | Response size monitoring | Pagination, compression           |

**Database Optimization Checklist:**

```sql
-- Check for missing indexes
SELECT
  schemaname, tablename,
  seq_scan, seq_tup_read,
  idx_scan, idx_tup_fetch
FROM pg_stat_user_tables
WHERE seq_scan > idx_scan
ORDER BY seq_tup_read DESC;

-- Check for slow queries
SELECT query, calls, total_time, mean_time, rows
FROM pg_stat_statements
ORDER BY mean_time DESC
LIMIT 20;

-- Check for lock contention
SELECT blocked_locks.pid AS blocked_pid,
       blocking_locks.pid AS blocking_pid,
       blocked_activity.query AS blocked_query
FROM pg_catalog.pg_locks blocked_locks
JOIN pg_catalog.pg_locks blocking_locks
  ON blocking_locks.locktype = blocked_locks.locktype
WHERE NOT blocked_locks.granted;
```

## Best Practices

1. **Test in Production-like Environments**
   - Match hardware specifications
   - Use realistic data volumes
   - Simulate actual traffic patterns

2. **Establish Baselines First**
   - Measure current performance before optimizing
   - Document baseline metrics
   - Track changes over time

3. **Test Early and Often**
   - Include performance tests in CI/CD
   - Catch regressions before deployment
   - Monitor trends, not just thresholds

4. **Use Realistic Test Data**
   - Volume should match production
   - Include edge cases (large records, unicode)
   - Test with cold and warm caches

5. **Monitor in Production**
   - Performance testing complements, not replaces monitoring
   - Use APM tools for real user metrics
   - Alert on performance degradation

6. **Document and Share Results**
   - Keep performance test reports
   - Share findings with the team
   - Track optimization improvements

## Examples

### Example: Complete k6 Load Test Suite

```javascript
// k6/scenarios/api-load-test.js
import http from "k6/http";
import { check, group, sleep } from "k6";
import { Rate, Trend, Counter } from "k6/metrics";

// Custom metrics
const errorRate = new Rate("errors");
const orderLatency = new Trend("order_latency");
const ordersCreated = new Counter("orders_created");

// Test configuration
export const options = {
  scenarios: {
    // Constant load for baseline
    baseline: {
      executor: "constant-vus",
      vus: 10,
      duration: "5m",
      tags: { scenario: "baseline" },
    },
    // Ramping load for stress test
    stress: {
      executor: "ramping-vus",
      startVUs: 0,
      stages: [
        { duration: "2m", target: 50 },
        { duration: "5m", target: 50 },
        { duration: "2m", target: 100 },
        { duration: "5m", target: 100 },
        { duration: "2m", target: 0 },
      ],
      startTime: "5m",
      tags: { scenario: "stress" },
    },
    // Spike test
    spike: {
      executor: "ramping-vus",
      startVUs: 0,
      stages: [
        { duration: "10s", target: 200 },
        { duration: "1m", target: 200 },
        { duration: "10s", target: 0 },
      ],
      startTime: "20m",
      tags: { scenario: "spike" },
    },
  },
  thresholds: {
    http_req_duration: ["p(95)<500", "p(99)<1500"],
    errors: ["rate<0.01"],
    order_latency: ["p(95)<2000"],
  },
};

const BASE_URL = __ENV.BASE_URL || "http://localhost:3000";

export function setup() {
  // Login and get auth token
  const loginRes = http.post(
    `${BASE_URL}/api/auth/login`,
    JSON.stringify({
      email: "loadtest@example.com",
      password: "loadtest123",
    }),
    {
      headers: { "Content-Type": "application/json" },
    },
  );

  return { token: loginRes.json("token") };
}

export default function (data) {
  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${data.token}`,
  };

  group("Browse Products", () => {
    const productsRes = http.get(`${BASE_URL}/api/products`, { headers });
    check(productsRes, {
      "products status 200": (r) => r.status === 200,
      "products returned": (r) => r.json("data").length > 0,
    });
    errorRate.add(productsRes.status !== 200);
    sleep(1);
  });

  group("View Product Detail", () => {
    const productRes = http.get(`${BASE_URL}/api/products/1`, { headers });
    check(productRes, {
      "product status 200": (r) => r.status === 200,
    });
    errorRate.add(productRes.status !== 200);
    sleep(0.5);
  });

  group("Create Order", () => {
    const start = Date.now();
    const orderRes = http.post(
      `${BASE_URL}/api/orders`,
      JSON.stringify({
        items: [{ productId: 1, quantity: 1 }],
      }),
      { headers },
    );

    orderLatency.add(Date.now() - start);

    const success = check(orderRes, {
      "order status 201": (r) => r.status === 201,
      "order has id": (r) => r.json("id") !== undefined,
    });

    if (success) ordersCreated.add(1);
    errorRate.add(!success);
    sleep(2);
  });
}

export function teardown(data) {
  // Cleanup test data if needed
  console.log("Test completed");
}
```

### Example: Performance Monitoring Dashboard Config

```yaml
# grafana/dashboards/performance.json (simplified)
panels:
  - title: "Request Latency (p95)"
    type: graph
    targets:
      - expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

  - title: "Throughput (RPS)"
    type: graph
    targets:
      - expr: rate(http_requests_total[1m])

  - title: "Error Rate"
    type: graph
    targets:
      - expr: rate(http_requests_total{status_code=~"5.."}[5m]) / rate(http_requests_total[5m])

  - title: "Active Connections"
    type: gauge
    targets:
      - expr: active_connections

alerts:
  - name: HighLatency
    expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 0.5
    for: 5m
    labels:
      severity: warning

  - name: HighErrorRate
    expr: rate(http_requests_total{status_code=~"5.."}[5m]) / rate(http_requests_total[5m]) > 0.01
    for: 2m
    labels:
      severity: critical
```
