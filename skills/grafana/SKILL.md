---
name: grafana
description: Observability visualization with Grafana and the LGTM stack (Loki, Grafana, Tempo, Mimir). Use when implementing dashboards, log aggregation, distributed tracing, or metrics visualization. Triggers: grafana, loki, tempo, mimir, dashboard, logql, traceql, observability stack, LGTM.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# Grafana and LGTM Stack Skill

## Overview

The LGTM stack provides a complete observability solution:

- **Loki**: Log aggregation and querying (LogQL)
- **Grafana**: Visualization and dashboarding
- **Tempo**: Distributed tracing (TraceQL)
- **Mimir**: Long-term metrics storage (Prometheus-compatible)

This skill covers setup, configuration, dashboard creation, querying, and alerting for production observability.

## LGTM Stack Components

### Loki - Log Aggregation

#### Architecture - Loki

Horizontally scalable log aggregation inspired by Prometheus

- Indexes only metadata (labels), not log content
- Cost-effective storage with object stores (S3, GCS, etc.)
- LogQL query language similar to PromQL

#### Key Concepts - Loki

- Labels for indexing (low cardinality)
- Log streams identified by unique label sets
- Parsers: logfmt, JSON, regex, pattern
- Line filters and label filters

### Grafana - Visualization

#### Features

- Multi-datasource dashboarding
- Templating and variables
- Alerting (unified alerting)
- Dashboard provisioning
- Role-based access control (RBAC)
- Explore mode for ad-hoc queries

### Tempo - Distributed Tracing

#### Architecture - Tempo

Scalable distributed tracing backend

- Cost-effective trace storage
- TraceQL for trace querying
- Integration with logs and metrics (trace-to-logs, trace-to-metrics)
- OpenTelemetry compatible

### Mimir - Metrics Storage

#### Architecture - Mimir

Horizontally scalable long-term Prometheus storage

- Multi-tenancy support
- Query federation
- High availability
- Prometheus remote_write compatible

## Setup and Configuration

### Loki Configuration

#### Production Loki Config

File: `loki.yaml`

```yaml
auth_enabled: false

server:
  http_listen_port: 3100
  grpc_listen_port: 9096
  log_level: info

common:
  path_prefix: /loki
  storage:
    filesystem:
      chunks_directory: /loki/chunks
      rules_directory: /loki/rules
  replication_factor: 1
  ring:
    kvstore:
      store: inmemory

schema_config:
  configs:
    - from: 2024-01-01
      store: tsdb
      object_store: s3
      schema: v13
      index:
        prefix: index_
        period: 24h

storage_config:
  aws:
    s3: s3://us-east-1/my-loki-bucket
    s3forcepathstyle: true
  tsdb_shipper:
    active_index_directory: /loki/tsdb-index
    cache_location: /loki/tsdb-cache
    shared_store: s3

limits_config:
  retention_period: 744h # 31 days
  ingestion_rate_mb: 10
  ingestion_burst_size_mb: 20
  max_query_series: 500
  max_query_lookback: 30d
  max_streams_per_user: 0
  max_global_streams_per_user: 5000
  reject_old_samples: true
  reject_old_samples_max_age: 168h

compactor:
  working_directory: /loki/compactor
  shared_store: s3
  compaction_interval: 10m
  retention_enabled: true
  retention_delete_delay: 2h
  retention_delete_worker_count: 150

querier:
  max_concurrent: 4

query_range:
  align_queries_with_step: true
  cache_results: true
  results_cache:
    cache:
      embedded_cache:
        enabled: true
        max_size_mb: 100
```

#### Multi-tenant Loki Config

With authentication

```yaml
auth_enabled: true
server:
  http_listen_port: 3100

common:
  storage:
    s3:
      endpoint: s3.amazonaws.com
      bucketnames: loki-chunks
      region: us-east-1
      access_key_id: ${AWS_ACCESS_KEY_ID}
      secret_access_key: ${AWS_SECRET_ACCESS_KEY}

distributor:
  ring:
    kvstore:
      store: memberlist

ingester:
  lifecycler:
    ring:
      kvstore:
        store: memberlist
      replication_factor: 3
  chunk_idle_period: 30m
  chunk_block_size: 262144
  chunk_retain_period: 1m
  max_transfer_retries: 0

memberlist:
  join_members:
    - loki-gossip-ring.loki.svc.cluster.local:7946
```

### Tempo Configuration

#### Production Tempo Config

File: `tempo.yaml`

```yaml
server:
  http_listen_port: 3200
  grpc_listen_port: 9096

distributor:
  receivers:
    otlp:
      protocols:
        http:
        grpc:
    jaeger:
      protocols:
        thrift_http:
        grpc:

ingester:
  max_block_duration: 5m

compactor:
  compaction:
    block_retention: 720h # 30 days

storage:
  trace:
    backend: s3
    s3:
      bucket: tempo-traces
      endpoint: s3.amazonaws.com
      region: us-east-1
    pool:
      max_workers: 100
      queue_depth: 10000
    wal:
      path: /var/tempo/wal

query_frontend:
  search:
    duration_slo: 5s
    throughput_bytes_slo: 1.073741824e+09
  trace_by_id:
    duration_slo: 5s

metrics_generator:
  registry:
    external_labels:
      source: tempo
      cluster: primary
  storage:
    path: /var/tempo/generator/wal
    remote_write:
      - url: http://mimir:9009/api/v1/push
        send_exemplars: true

overrides:
  defaults:
    metrics_generator:
      processors: [service-graphs, span-metrics]
```

### Grafana Data Source Configuration

#### Loki Data Source

File: `loki-datasource.yaml`

```yaml
apiVersion: 1

datasources:
  - name: Loki
    type: loki
    access: proxy
    url: http://loki:3100
    jsonData:
      maxLines: 1000
      derivedFields:
        - datasourceUid: tempo_uid
          matcherRegex: "trace_id=(\\w+)"
          name: TraceID
          url: "$${__value.raw}"
    editable: false
```

#### Tempo Data Source

File: `tempo-datasource.yaml`

```yaml
apiVersion: 1

datasources:
  - name: Tempo
    type: tempo
    access: proxy
    url: http://tempo:3200
    uid: tempo_uid
    jsonData:
      httpMethod: GET
      tracesToLogs:
        datasourceUid: loki_uid
        tags: ["job", "instance", "pod", "namespace"]
        mappedTags: [{ key: "service.name", value: "service" }]
        mapTagNamesEnabled: false
        spanStartTimeShift: "1h"
        spanEndTimeShift: "1h"
        filterByTraceID: false
        filterBySpanID: false
      tracesToMetrics:
        datasourceUid: prometheus_uid
        tags: [{ key: "service.name", value: "service" }]
        queries:
          - name: "Sample query"
            query: "sum(rate(tempo_spanmetrics_latency_bucket{$__tags}[5m]))"
      serviceMap:
        datasourceUid: prometheus_uid
      search:
        hide: false
      nodeGraph:
        enabled: true
      lokiSearch:
        datasourceUid: loki_uid
    editable: false
```

#### Mimir/Prometheus Data Source

File: `mimir-datasource.yaml`

```yaml
apiVersion: 1

datasources:
  - name: Mimir
    type: prometheus
    access: proxy
    url: http://mimir:8080/prometheus
    uid: prometheus_uid
    jsonData:
      httpMethod: POST
      exemplarTraceIdDestinations:
        - datasourceUid: tempo_uid
          name: trace_id
      prometheusType: Mimir
      prometheusVersion: 2.40.0
      cacheLevel: "High"
      incrementalQuerying: true
      incrementalQueryOverlapWindow: 10m
    editable: false
```

## LogQL Query Patterns

### Basic Log Queries

#### Stream Selection

```logql
# Simple label matching
{namespace="production", app="api"}

# Regex matching
{app=~"api|web|worker"}

# Not equal
{env!="staging"}

# Multiple conditions
{namespace="production", app="api", level!="debug"}
```

#### Line Filters

```logql
# Contains
{app="api"} |= "error"

# Does not contain
{app="api"} != "debug"

# Regex match
{app="api"} |~ "error|exception|fatal"

# Case insensitive
{app="api"} |~ "(?i)error"

# Chaining filters
{app="api"} |= "error" != "timeout"
```

### Parsing and Extraction

#### JSON Parsing

```logql
# Parse JSON logs
{app="api"} | json

# Extract specific fields
{app="api"} | json message="msg", level="severity"

# Filter on extracted field
{app="api"} | json | level="error"

# Nested JSON
{app="api"} | json | line_format "{{.response.status}}"
```

#### Logfmt Parsing

```logql
# Parse logfmt (key=value)
{app="api"} | logfmt

# Extract specific fields
{app="api"} | logfmt level, caller, msg

# Filter parsed fields
{app="api"} | logfmt | level="error"
```

#### Pattern Parsing

```logql
# Extract with pattern
{app="nginx"} | pattern `<ip> - - <_> "<method> <uri> <_>" <status> <_>`

# Filter on extracted values
{app="nginx"} | pattern `<_> <status> <_>` | status >= 400

# Complex pattern
{app="api"} | pattern `level=<level> msg="<msg>" duration=<duration>ms`
```

#### Regex Parsing

```logql
# Named capture groups
{app="api"} | regexp `(?P<method>[A-Z]+) (?P<uri>/[^ ]*) (?P<duration>[0-9.]+)ms`

# Multiple regexes
{app="api"}
  | regexp `level=(?P<level>[a-z]+)`
  | regexp `duration=(?P<duration>[0-9.]+)ms`
```

### Label Formatting and Manipulation

#### Label Format

```logql
# Add labels from parsed fields
{app="api"} | json | label_format level=severity

# Rename labels
{app="api"} | label_format environment=env

# Template labels
{app="api"} | json | label_format full_path=`{{.namespace}}/{{.pod}}`

# Convert to lowercase
{app="api"} | label_format app=`{{ ToLower .app }}`
```

#### Line Format

```logql
# Custom output format
{app="api"} | json | line_format "{{.timestamp}} [{{.level}}] {{.message}}"

# Only specific fields
{app="api"} | logfmt | line_format "{{.msg}}"

# Conditional formatting
{app="api"} | json
  | line_format `{{if eq .level "error"}}ERROR: {{end}}{{.msg}}`
```

### Aggregations and Metrics

#### Count Queries

```logql
# Count log lines over time
count_over_time({app="api"}[5m])

# Rate of logs
rate({app="api"}[5m])

# Errors per second
sum(rate({app="api"} |= "error" [5m])) by (namespace)

# Error ratio
sum(rate({app="api"} |= "error" [5m]))
/
sum(rate({app="api"}[5m]))
```

#### Extracted Metrics

```logql
# Parse and aggregate numeric values
sum(rate({app="api"} | json | __error__="" [5m])) by (status_code)

# Average duration
avg_over_time({app="api"}
  | logfmt
  | unwrap duration [5m]) by (endpoint)

# P95 latency
quantile_over_time(0.95, {app="api"}
  | regexp `duration=(?P<duration>[0-9.]+)ms`
  | unwrap duration [5m]) by (method)

# Bytes processed
sum(bytes_over_time({app="api"}[5m]))
```

#### Advanced Aggregations

```logql
# Top 10 error messages
topk(10,
  sum by (msg) (
    count_over_time({app="api"}
      | json
      | level="error" [1h]
    )
  )
)

# Request rate by endpoint
sum by (endpoint) (
  rate({app="api"}
    | json
    | __error__="" [5m]
  )
)

# 4xx/5xx rate
sum(
  rate({app="api"}
    | pattern `<_> <status> <_>`
    | status >= 400 [5m]
  )
) by (status)
```

### Production Query Examples

#### Error Investigation

```logql
# Find errors with context
{namespace="production", app="api"}
  | json
  | level="error"
  | line_format "{{.timestamp}} [{{.trace_id}}] {{.message}}"

# Errors with stack traces
{app="api"}
  |= "error"
  |= "stack"
  | json
  | line_format "{{.message}}\n{{.stack}}"

# Group errors by type
sum by (error_type) (
  count_over_time({app="api"}
    | json
    | level="error" [1h]
  )
)
```

#### Performance Analysis

```logql
# Slow requests (>1s)
{app="api"}
  | logfmt
  | unwrap duration
  | duration > 1000
  | line_format "{{.method}} {{.uri}} took {{.duration}}ms"

# P99 latency by endpoint
quantile_over_time(0.99, {app="api"}
  | json
  | unwrap response_time [5m]) by (endpoint)

# Request throughput
sum(rate({app="api"} | json | __error__="" [1m])) by (method, endpoint)
```

## TraceQL Query Patterns

### Basic Trace Queries

#### Span Search

```traceql
# Find traces by service
{ .service.name = "api" }

# Find by span name
{ name = "GET /users" }

# HTTP status codes
{ .http.status_code = 500 }

# Combine conditions
{ .service.name = "api" && .http.status_code >= 400 }

# Duration filter (in nanoseconds)
{ duration > 1s }
```

#### Resource and Span Attributes

```traceql
# Resource attributes (service-level)
{ resource.service.name = "checkout" }
{ resource.deployment.environment = "production" }

# Span attributes
{ span.http.method = "POST" }
{ span.db.system = "postgresql" }

# Intrinsic fields
{ name = "database-query" && duration > 100ms }
{ kind = "server" }
{ status = "error" }
```

### Advanced TraceQL

#### Aggregations

```traceql
# Count spans
{ .service.name = "api" } | count() > 10

# Min/Max/Avg duration
{ .service.name = "api" } | max(duration) > 5s

# Structural operators
{ .service.name = "api" }
  >> { .service.name = "database" && duration > 100ms }
```

#### Span Sets and Relationships

```traceql
# Parent-child relationship
{ .service.name = "frontend" }
  >> { .service.name = "backend" && .http.status_code = 500 }

# Sibling spans
{ .service.name = "api" }
  && { .service.name = "cache" }

# Descendant spans
{ .service.name = "api" }
  >>+ { .db.system = "postgresql" && duration > 1s }
```

#### Production TraceQL Examples

```traceql
# Failed database queries
{ .service.name = "api" }
  >> { .db.system = "postgresql" && status = "error" }

# Slow checkout flows
{ name = "checkout" && duration > 5s }

# Cache misses leading to slow DB calls
{ .cache.hit = false }
  >> { .db.system = "postgresql" && duration > 500ms }

# Error traces with specific tag
{ status = "error" && .tenant.id = "acme-corp" }

# High cardinality debugging
{ .service.name = "api" && .user.id = "user123" }
```

## Dashboard Design Best Practices

### Dashboard Organization

1. Hierarchy: Overview -> Service -> Component -> Deep Dive
2. Golden Signals: Latency, Traffic, Errors, Saturation
3. Variable-driven: Use templates for flexibility
4. Consistent Layouts: Grid alignment, logical flow
5. Performance: Limit queries, use query caching

### Panel Best Practices

- **Titles**: Clear, descriptive (include units)
- **Legends**: Show only when needed, use `{{label}}` format
- **Axes**: Label with units, appropriate ranges
- **Thresholds**: Use for visual cues (green/yellow/red)
- **Links**: Deep-link to related dashboards/logs/traces

### Variable Patterns

#### Common Variables

```json
{
  "templating": {
    "list": [
      {
        "name": "datasource",
        "type": "datasource",
        "query": "prometheus"
      },
      {
        "name": "namespace",
        "type": "query",
        "datasource": "${datasource}",
        "query": "label_values(kube_pod_info, namespace)",
        "multi": true,
        "includeAll": true
      },
      {
        "name": "pod",
        "type": "query",
        "datasource": "${datasource}",
        "query": "label_values(kube_pod_info{namespace=~\"$namespace\"}, pod)",
        "multi": true,
        "includeAll": true
      },
      {
        "name": "interval",
        "type": "interval",
        "auto": true,
        "auto_count": 30,
        "auto_min": "10s",
        "options": ["1m", "5m", "15m", "30m", "1h", "6h", "12h", "1d"]
      }
    ]
  }
}
```

## Complete Dashboard Examples

### Application Observability Dashboard

```json
{
  "dashboard": {
    "title": "Application Observability - ${app}",
    "tags": ["observability", "application"],
    "timezone": "browser",
    "editable": true,
    "graphTooltip": 1,
    "time": {
      "from": "now-1h",
      "to": "now"
    },
    "templating": {
      "list": [
        {
          "name": "app",
          "type": "query",
          "datasource": "Mimir",
          "query": "label_values(up, app)",
          "current": {
            "selected": false,
            "text": "api",
            "value": "api"
          }
        },
        {
          "name": "namespace",
          "type": "query",
          "datasource": "Mimir",
          "query": "label_values(up{app=\"$app\"}, namespace)",
          "multi": true,
          "includeAll": true
        }
      ]
    },
    "panels": [
      {
        "id": 1,
        "title": "Request Rate",
        "type": "graph",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total{app=\"$app\", namespace=~\"$namespace\"}[$__rate_interval])) by (method, status)",
            "legendFormat": "{{method}} - {{status}}"
          }
        ],
        "gridPos": {
          "h": 8,
          "w": 12,
          "x": 0,
          "y": 0
        },
        "yaxes": [
          {
            "format": "reqps",
            "label": "Requests/sec"
          }
        ]
      },
      {
        "id": 2,
        "title": "P95 Latency",
        "type": "graph",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket{app=\"$app\", namespace=~\"$namespace\"}[$__rate_interval])) by (le, endpoint))",
            "legendFormat": "{{endpoint}}"
          }
        ],
        "gridPos": {
          "h": 8,
          "w": 12,
          "x": 12,
          "y": 0
        },
        "yaxes": [
          {
            "format": "s",
            "label": "Duration"
          }
        ],
        "thresholds": [
          {
            "value": 1,
            "colorMode": "critical",
            "fill": true,
            "line": true,
            "op": "gt"
          }
        ]
      },
      {
        "id": 3,
        "title": "Error Rate",
        "type": "graph",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total{app=\"$app\", namespace=~\"$namespace\", status=~\"5..\"}[$__rate_interval])) / sum(rate(http_requests_total{app=\"$app\", namespace=~\"$namespace\"}[$__rate_interval]))",
            "legendFormat": "Error %"
          }
        ],
        "gridPos": {
          "h": 8,
          "w": 12,
          "x": 0,
          "y": 8
        },
        "yaxes": [
          {
            "format": "percentunit",
            "max": 1,
            "min": 0
          }
        ],
        "alert": {
          "conditions": [
            {
              "evaluator": {
                "params": [0.01],
                "type": "gt"
              },
              "operator": {
                "type": "and"
              },
              "query": {
                "params": ["A", "5m", "now"]
              },
              "reducer": {
                "type": "avg"
              },
              "type": "query"
            }
          ],
          "frequency": "1m",
          "handler": 1,
          "name": "Error Rate Alert",
          "noDataState": "no_data",
          "notifications": []
        }
      },
      {
        "id": 4,
        "title": "Recent Error Logs",
        "type": "logs",
        "datasource": "Loki",
        "targets": [
          {
            "expr": "{app=\"$app\", namespace=~\"$namespace\"} | json | level=\"error\"",
            "refId": "A"
          }
        ],
        "options": {
          "showTime": true,
          "showLabels": false,
          "showCommonLabels": false,
          "wrapLogMessage": true,
          "dedupStrategy": "none",
          "enableLogDetails": true
        },
        "gridPos": {
          "h": 8,
          "w": 12,
          "x": 12,
          "y": 8
        }
      },
      {
        "id": 5,
        "title": "Trace Error Rate",
        "type": "graph",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "sum(rate(traces_spanmetrics_calls_total{service=\"$app\", status_code=\"STATUS_CODE_ERROR\"}[$__rate_interval])) / sum(rate(traces_spanmetrics_calls_total{service=\"$app\"}[$__rate_interval]))",
            "legendFormat": "Trace Error %"
          }
        ],
        "gridPos": {
          "h": 6,
          "w": 8,
          "x": 0,
          "y": 16
        }
      },
      {
        "id": 6,
        "title": "Active Traces",
        "type": "stat",
        "datasource": "Tempo",
        "targets": [
          {
            "queryType": "traceql",
            "query": "{ resource.service.name = \"$app\" }",
            "refId": "A"
          }
        ],
        "options": {
          "graphMode": "area",
          "colorMode": "value",
          "justifyMode": "auto",
          "textMode": "auto"
        },
        "gridPos": {
          "h": 6,
          "w": 8,
          "x": 8,
          "y": 16
        }
      },
      {
        "id": 7,
        "title": "Top Endpoints by Request Count",
        "type": "table",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "topk(10, sum by (endpoint) (rate(http_requests_total{app=\"$app\", namespace=~\"$namespace\"}[$__rate_interval])))",
            "format": "table",
            "instant": true
          }
        ],
        "transformations": [
          {
            "id": "organize",
            "options": {
              "excludeByName": {
                "Time": true
              },
              "renameByName": {
                "endpoint": "Endpoint",
                "Value": "Req/s"
              }
            }
          }
        ],
        "gridPos": {
          "h": 6,
          "w": 8,
          "x": 16,
          "y": 16
        }
      }
    ],
    "links": [
      {
        "title": "Explore Logs",
        "url": "/explore?left={\"datasource\":\"Loki\",\"queries\":[{\"expr\":\"{app=\\\"$app\\\",namespace=~\\\"$namespace\\\"}\"}]}",
        "type": "link",
        "icon": "doc"
      },
      {
        "title": "Explore Traces",
        "url": "/explore?left={\"datasource\":\"Tempo\",\"queries\":[{\"query\":\"{resource.service.name=\\\"$app\\\"}\",\"queryType\":\"traceql\"}]}",
        "type": "link",
        "icon": "gf-traces"
      }
    ]
  }
}
```

### Log Analysis Dashboard

```json
{
  "dashboard": {
    "title": "Log Analysis - ${namespace}",
    "tags": ["logs", "loki"],
    "templating": {
      "list": [
        {
          "name": "namespace",
          "type": "query",
          "datasource": "Loki",
          "query": "label_values(namespace)",
          "multi": false
        },
        {
          "name": "app",
          "type": "query",
          "datasource": "Loki",
          "query": "label_values({namespace=\"$namespace\"}, app)",
          "multi": true,
          "includeAll": true
        },
        {
          "name": "level",
          "type": "custom",
          "options": [
            { "text": "All", "value": ".*" },
            { "text": "Error", "value": "error" },
            { "text": "Warning", "value": "warn|warning" },
            { "text": "Info", "value": "info" },
            { "text": "Debug", "value": "debug" }
          ],
          "current": {
            "text": "All",
            "value": ".*"
          }
        }
      ]
    },
    "panels": [
      {
        "id": 1,
        "title": "Log Volume",
        "type": "graph",
        "datasource": "Loki",
        "targets": [
          {
            "expr": "sum by (app) (rate({namespace=\"$namespace\", app=~\"$app\"} | json | level=~\"$level\" [$__interval]))",
            "legendFormat": "{{app}}"
          }
        ],
        "gridPos": { "h": 8, "w": 24, "x": 0, "y": 0 }
      },
      {
        "id": 2,
        "title": "Log Level Distribution",
        "type": "piechart",
        "datasource": "Loki",
        "targets": [
          {
            "expr": "sum by (level) (count_over_time({namespace=\"$namespace\", app=~\"$app\"} | json | level=~\"$level\" [$__range]))",
            "legendFormat": "{{level}}"
          }
        ],
        "options": {
          "legend": {
            "displayMode": "table",
            "placement": "right",
            "values": ["value", "percent"]
          }
        },
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": 8 }
      },
      {
        "id": 3,
        "title": "Top Error Messages",
        "type": "bargauge",
        "datasource": "Loki",
        "targets": [
          {
            "expr": "topk(10, sum by (msg) (count_over_time({namespace=\"$namespace\", app=~\"$app\"} | json | level=\"error\" [$__range])))",
            "legendFormat": "{{msg}}"
          }
        ],
        "options": {
          "orientation": "horizontal",
          "displayMode": "gradient"
        },
        "gridPos": { "h": 8, "w": 12, "x": 12, "y": 8 }
      },
      {
        "id": 4,
        "title": "Log Stream",
        "type": "logs",
        "datasource": "Loki",
        "targets": [
          {
            "expr": "{namespace=\"$namespace\", app=~\"$app\"} | json | level=~\"$level\"",
            "refId": "A"
          }
        ],
        "options": {
          "showTime": true,
          "showLabels": true,
          "wrapLogMessage": true,
          "dedupStrategy": "none",
          "enableLogDetails": true
        },
        "gridPos": { "h": 12, "w": 24, "x": 0, "y": 16 }
      }
    ]
  }
}
```

### Distributed Tracing Dashboard

```json
{
  "dashboard": {
    "title": "Distributed Tracing - ${service}",
    "tags": ["tracing", "tempo"],
    "templating": {
      "list": [
        {
          "name": "service",
          "type": "query",
          "datasource": "Tempo",
          "query": "service.name",
          "multi": false
        },
        {
          "name": "min_duration",
          "type": "custom",
          "options": [
            { "text": "Any", "value": "0" },
            { "text": ">100ms", "value": "100ms" },
            { "text": ">500ms", "value": "500ms" },
            { "text": ">1s", "value": "1s" },
            { "text": ">5s", "value": "5s" }
          ]
        }
      ]
    },
    "panels": [
      {
        "id": 1,
        "title": "Request Rate (from span metrics)",
        "type": "graph",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "sum(rate(traces_spanmetrics_calls_total{service=\"$service\"}[$__rate_interval])) by (span_name)",
            "legendFormat": "{{span_name}}"
          }
        ],
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": 0 }
      },
      {
        "id": 2,
        "title": "P95 Latency (from span metrics)",
        "type": "graph",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(traces_spanmetrics_latency_bucket{service=\"$service\"}[$__rate_interval])) by (le, span_name))",
            "legendFormat": "{{span_name}}"
          }
        ],
        "gridPos": { "h": 8, "w": 12, "x": 12, "y": 0 }
      },
      {
        "id": 3,
        "title": "Error Rate",
        "type": "stat",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "sum(rate(traces_spanmetrics_calls_total{service=\"$service\", status_code=\"STATUS_CODE_ERROR\"}[$__rate_interval])) / sum(rate(traces_spanmetrics_calls_total{service=\"$service\"}[$__rate_interval]))"
          }
        ],
        "options": {
          "reduceOptions": {
            "values": false,
            "calcs": ["lastNotNull"]
          },
          "graphMode": "area",
          "colorMode": "background",
          "textMode": "value_and_name"
        },
        "fieldConfig": {
          "defaults": {
            "unit": "percentunit",
            "thresholds": {
              "mode": "absolute",
              "steps": [
                { "value": 0, "color": "green" },
                { "value": 0.01, "color": "yellow" },
                { "value": 0.05, "color": "red" }
              ]
            }
          }
        },
        "gridPos": { "h": 4, "w": 6, "x": 0, "y": 8 }
      },
      {
        "id": 4,
        "title": "Traces with Errors",
        "type": "table",
        "datasource": "Tempo",
        "targets": [
          {
            "queryType": "traceql",
            "query": "{ resource.service.name = \"$service\" && status = error }",
            "refId": "A",
            "limit": 20
          }
        ],
        "transformations": [
          {
            "id": "organize",
            "options": {
              "renameByName": {
                "traceID": "Trace ID",
                "startTime": "Start Time",
                "duration": "Duration"
              }
            }
          }
        ],
        "gridPos": { "h": 8, "w": 18, "x": 6, "y": 8 }
      },
      {
        "id": 5,
        "title": "Service Dependency Graph",
        "type": "nodeGraph",
        "datasource": "Mimir",
        "targets": [
          {
            "expr": "traces_service_graph_request_total",
            "format": "table"
          }
        ],
        "gridPos": { "h": 12, "w": 24, "x": 0, "y": 16 }
      }
    ]
  }
}
```

## Grafana Alerting

### Alert Rule Configuration

#### Prometheus/Mimir Alert Rule

```yaml
apiVersion: 1

groups:
  - name: application_alerts
    interval: 1m
    rules:
      - uid: error_rate_high
        title: High Error Rate
        condition: A
        data:
          - refId: A
            queryType: ""
            relativeTimeRange:
              from: 300
              to: 0
            datasourceUid: prometheus_uid
            model:
              expr: |
                sum(rate(http_requests_total{status=~"5.."}[5m]))
                /
                sum(rate(http_requests_total[5m]))
                > 0.05
              intervalMs: 1000
              maxDataPoints: 43200
        noDataState: NoData
        execErrState: Error
        for: 5m
        annotations:
          description: 'Error rate is {{ printf "%.2f" $values.A.Value }}% (threshold: 5%)'
          summary: Application error rate is above threshold
        labels:
          severity: critical
          team: platform
        isPaused: false

      - uid: high_latency
        title: High P95 Latency
        condition: A
        data:
          - refId: A
            datasourceUid: prometheus_uid
            model:
              expr: |
                histogram_quantile(0.95,
                  sum(rate(http_request_duration_seconds_bucket[5m])) by (le, endpoint)
                ) > 2
        for: 10m
        annotations:
          description: "P95 latency is {{ $values.A.Value }}s on endpoint {{ $labels.endpoint }}"
          runbook_url: https://wiki.company.com/runbooks/high-latency
        labels:
          severity: warning

      - uid: pod_restart_high
        title: High Pod Restart Rate
        condition: A
        data:
          - refId: A
            datasourceUid: prometheus_uid
            model:
              expr: |
                rate(kube_pod_container_status_restarts_total[15m]) > 0.1
        for: 5m
        annotations:
          description: "Pod {{ $labels.pod }} in namespace {{ $labels.namespace }} is restarting frequently"
        labels:
          severity: warning
          team: sre
```

#### Loki Alert Rule

```yaml
apiVersion: 1

groups:
  - name: log_based_alerts
    interval: 1m
    rules:
      - uid: error_spike
        title: Error Log Spike
        condition: A
        data:
          - refId: A
            queryType: ""
            datasourceUid: loki_uid
            model:
              expr: |
                sum(rate({app="api"} | json | level="error" [5m]))
                > 10
        for: 2m
        annotations:
          description: "Error log rate is {{ $values.A.Value }} logs/sec"
          summary: Spike in error logs detected
        labels:
          severity: warning

      - uid: critical_error_pattern
        title: Critical Error Pattern Detected
        condition: A
        data:
          - refId: A
            datasourceUid: loki_uid
            model:
              expr: |
                sum(count_over_time({app="api"}
                  |~ "OutOfMemoryError|StackOverflowError|FatalException" [5m]
                )) > 0
        for: 0m
        annotations:
          description: "Critical error pattern found in logs"
        labels:
          severity: critical
          page: true

      - uid: database_connection_errors
        title: Database Connection Errors
        condition: A
        data:
          - refId: A
            datasourceUid: loki_uid
            model:
              expr: |
                sum by (app) (
                  rate({namespace="production"}
                    | json
                    | message =~ ".*database connection.*failed.*" [5m]
                  )
                ) > 1
        for: 3m
        annotations:
          description: "App {{ $labels.app }} experiencing DB connection issues"
        labels:
          severity: critical
```

### Contact Points and Notification Policies

#### Contact Point Configuration

```yaml
apiVersion: 1

contactPoints:
  - orgId: 1
    name: slack-critical
    receivers:
      - uid: slack_critical
        type: slack
        settings:
          url: https://hooks.slack.com/services/YOUR/WEBHOOK/URL
          title: "{{ .GroupLabels.alertname }}"
          text: |
            {{ range .Alerts }}
            *Alert:* {{ .Labels.alertname }}
            *Summary:* {{ .Annotations.summary }}
            *Description:* {{ .Annotations.description }}
            *Severity:* {{ .Labels.severity }}
            {{ end }}
        disableResolveMessage: false

  - orgId: 1
    name: pagerduty-oncall
    receivers:
      - uid: pagerduty_oncall
        type: pagerduty
        settings:
          integrationKey: YOUR_INTEGRATION_KEY
          severity: critical
          class: infrastructure
          component: kubernetes

  - orgId: 1
    name: email-team
    receivers:
      - uid: email_team
        type: email
        settings:
          addresses: team@company.com
          singleEmail: true

notificationPolicies:
  - orgId: 1
    receiver: slack-critical
    group_by: ["alertname", "namespace"]
    group_wait: 30s
    group_interval: 5m
    repeat_interval: 4h
    routes:
      - receiver: pagerduty-oncall
        matchers:
          - severity = critical
          - page = true
        group_wait: 10s
        repeat_interval: 1h
        continue: true

      - receiver: email-team
        matchers:
          - severity = warning
          - team = platform
        group_interval: 10m
        repeat_interval: 12h
```

## Dashboard Provisioning

### ConfigMap for Dashboard Provisioning

#### Kubernetes ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: grafana-dashboards
  namespace: monitoring
  labels:
    grafana_dashboard: "1"
data:
  application-dashboard.json: |
    {
      "dashboard": {
        "title": "Application Metrics",
        "uid": "app-metrics",
        "tags": ["application"],
        "timezone": "browser",
        "panels": []
      }
    }
  log-analysis.json: |
    {
      "dashboard": {
        "title": "Log Analysis",
        "uid": "log-analysis",
        "tags": ["logs"],
        "timezone": "browser",
        "panels": []
      }
    }
```

#### Dashboard Provider Configuration

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: grafana-dashboard-providers
  namespace: monitoring
data:
  dashboards.yaml: |
    apiVersion: 1
    providers:
      - name: 'default'
        orgId: 1
        folder: 'General'
        type: file
        disableDeletion: false
        updateIntervalSeconds: 10
        allowUiUpdates: true
        options:
          path: /etc/grafana/provisioning/dashboards

      - name: 'application'
        orgId: 1
        folder: 'Applications'
        type: file
        disableDeletion: true
        editable: false
        options:
          path: /var/lib/grafana/dashboards/application

      - name: 'infrastructure'
        orgId: 1
        folder: 'Infrastructure'
        type: file
        options:
          path: /var/lib/grafana/dashboards/infrastructure
```

#### Grafana Deployment with Provisioning

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: grafana
  namespace: monitoring
spec:
  replicas: 1
  selector:
    matchLabels:
      app: grafana
  template:
    metadata:
      labels:
        app: grafana
    spec:
      containers:
        - name: grafana
          image: grafana/grafana:10.2.0
          ports:
            - containerPort: 3000
          env:
            - name: GF_SECURITY_ADMIN_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: grafana-admin
                  key: password
            - name: GF_INSTALL_PLUGINS
              value: "grafana-piechart-panel,grafana-worldmap-panel"
          volumeMounts:
            - name: grafana-storage
              mountPath: /var/lib/grafana
            - name: grafana-datasources
              mountPath: /etc/grafana/provisioning/datasources
            - name: grafana-dashboards
              mountPath: /etc/grafana/provisioning/dashboards
            - name: dashboard-files
              mountPath: /var/lib/grafana/dashboards
      volumes:
        - name: grafana-storage
          persistentVolumeClaim:
            claimName: grafana-pvc
        - name: grafana-datasources
          configMap:
            name: grafana-datasources
        - name: grafana-dashboards
          configMap:
            name: grafana-dashboard-providers
        - name: dashboard-files
          configMap:
            name: grafana-dashboards
```

## GrafanaDashboard CRD (Grafana Operator)

### Using Grafana Operator

#### GrafanaDashboard Resource

```yaml
apiVersion: grafana.integreatly.org/v1beta1
kind: GrafanaDashboard
metadata:
  name: application-observability
  namespace: monitoring
  labels:
    app: grafana
spec:
  instanceSelector:
    matchLabels:
      dashboards: "grafana"

  # Inline dashboard JSON
  json: |
    {
      "dashboard": {
        "title": "Application Observability",
        "tags": ["generated", "application"],
        "timezone": "browser",
        "panels": [
          {
            "id": 1,
            "title": "Request Rate",
            "type": "graph",
            "datasource": "Prometheus",
            "targets": [
              {
                "expr": "sum(rate(http_requests_total[5m])) by (method)",
                "legendFormat": "{{method}}"
              }
            ]
          }
        ]
      }
    }
```

#### GrafanaDashboard from ConfigMap

```yaml
apiVersion: grafana.integreatly.org/v1beta1
kind: GrafanaDashboard
metadata:
  name: logs-dashboard
  namespace: monitoring
spec:
  instanceSelector:
    matchLabels:
      dashboards: "grafana"

  configMapRef:
    name: log-dashboard-config
    key: dashboard.json
```

#### Grafana Instance with Operator

```yaml
apiVersion: grafana.integreatly.org/v1beta1
kind: Grafana
metadata:
  name: grafana
  namespace: monitoring
  labels:
    dashboards: "grafana"
spec:
  config:
    log:
      mode: "console"
    auth:
      disable_login_form: false
    security:
      admin_user: admin
      admin_password: ${ADMIN_PASSWORD}

  deployment:
    spec:
      replicas: 2
      template:
        spec:
          containers:
            - name: grafana
              image: grafana/grafana:10.2.0

  service:
    spec:
      type: LoadBalancer
      ports:
        - name: http
          port: 3000
          targetPort: 3000

  ingress:
    spec:
      ingressClassName: nginx
      rules:
        - host: grafana.company.com
          http:
            paths:
              - path: /
                pathType: Prefix
                backend:
                  service:
                    name: grafana-service
                    port:
                      number: 3000
      tls:
        - hosts:
            - grafana.company.com
          secretName: grafana-tls
```

#### GrafanaDataSource CRD

```yaml
apiVersion: grafana.integreatly.org/v1beta1
kind: GrafanaDataSource
metadata:
  name: loki-datasource
  namespace: monitoring
spec:
  instanceSelector:
    matchLabels:
      dashboards: "grafana"

  datasource:
    name: Loki
    type: loki
    access: proxy
    url: http://loki-gateway.monitoring.svc.cluster.local:3100
    isDefault: false
    editable: false
    jsonData:
      maxLines: 1000
      derivedFields:
        - datasourceUid: tempo
          matcherRegex: "trace_id=(\\w+)"
          name: TraceID
          url: "$${__value.raw}"

---
apiVersion: grafana.integreatly.org/v1beta1
kind: GrafanaDataSource
metadata:
  name: tempo-datasource
  namespace: monitoring
spec:
  instanceSelector:
    matchLabels:
      dashboards: "grafana"

  datasource:
    name: Tempo
    type: tempo
    access: proxy
    url: http://tempo-query-frontend.monitoring.svc.cluster.local:3200
    uid: tempo
    jsonData:
      httpMethod: GET
      tracesToLogs:
        datasourceUid: loki
        tags: ["job", "instance", "pod", "namespace"]
        mappedTags: [{ key: "service.name", value: "service" }]
      nodeGraph:
        enabled: true
```

## Production Tips

### Performance Optimization

#### Query Optimization

- Use label filters before line filters
- Limit time ranges for expensive queries
- Use `unwrap` instead of parsing when possible
- Cache query results with query frontend

#### Dashboard Performance

- Limit number of panels (< 15 per dashboard)
- Use appropriate time intervals
- Avoid high-cardinality grouping
- Use `$__interval` for adaptive sampling

#### Storage Optimization

- Configure retention policies
- Use compaction for Loki and Tempo
- Implement tiered storage (hot/warm/cold)
- Monitor storage growth

### Security Best Practices

#### Authentication

- Enable auth (`auth_enabled: true` in Loki/Tempo)
- Use OAuth/LDAP for Grafana
- Implement multi-tenancy with org isolation

#### Authorization

- Configure RBAC in Grafana
- Limit datasource access by team
- Use folder permissions for dashboards

#### Network Security

- TLS for all components
- Network policies in Kubernetes
- Rate limiting at ingress

### Monitoring the Monitoring Stack

```promql
# Monitor Loki itself
sum(rate(loki_request_duration_seconds_count[5m])) by (route)

# Tempo ingestion rate
rate(tempo_distributor_spans_received_total[5m])

# Grafana active users
grafana_stat_active_users

# Query performance
histogram_quantile(0.99, sum(rate(loki_request_duration_seconds_bucket[5m])) by (le))
```

### Troubleshooting

#### Common Issues

1. High Cardinality: Too many unique label combinations
   - Solution: Reduce label dimensions, use log parsing instead

2. Query Timeouts: Complex queries on large datasets
   - Solution: Reduce time range, use aggregations, add query limits

3. Storage Growth: Unbounded retention
   - Solution: Configure retention policies, enable compaction

4. Missing Traces: Incomplete trace data
   - Solution: Check sampling rates, verify instrumentation, check network connectivity

#### Debugging Queries

```logql
# Check query performance
{app="api"} | json | __error__=""  # Filter parsing errors

# Verify labels exist
{namespace="production"} | logfmt | line_format "{{.level}}"

# Count parsing failures
sum(rate({app="api"} | json | __error__!="" [5m]))
```

## Resources

- [Loki Documentation](https://grafana.com/docs/loki/latest/)
- [Tempo Documentation](https://grafana.com/docs/tempo/latest/)
- [Grafana Documentation](https://grafana.com/docs/grafana/latest/)
- [LogQL Cheat Sheet](https://grafana.com/docs/loki/latest/logql/)
- [TraceQL Guide](https://grafana.com/docs/tempo/latest/traceql/)
- [Grafana Operator](https://github.com/grafana-operator/grafana-operator)
