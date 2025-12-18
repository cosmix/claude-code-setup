---
name: background-jobs
description: Background job processing patterns including job queues, scheduled jobs, worker pools, and retry strategies. Use when implementing async processing, Celery, Bull, Sidekiq, cron jobs, task queues, job monitoring, or worker management.
---

# Background Jobs

## Overview

Background jobs enable asynchronous processing of tasks outside the request-response cycle. This skill covers job queue patterns, scheduling, worker management, retry strategies, and monitoring for reliable task execution across different frameworks and languages.

## Key Concepts

### Job Queue Patterns

**Bull Queue (Node.js/Redis)**:

```typescript
import Queue, { Job, JobOptions } from "bull";
import { Redis } from "ioredis";

// Queue configuration
interface QueueConfig {
  name: string;
  redis: Redis;
  defaultJobOptions?: JobOptions;
}

// Job data interfaces
interface EmailJobData {
  to: string;
  subject: string;
  template: string;
  context: Record<string, unknown>;
}

interface ImageProcessingJobData {
  imageId: string;
  operations: Array<{
    type: "resize" | "crop" | "compress";
    params: Record<string, unknown>;
  }>;
}

// Queue factory
function createQueue<T>(config: QueueConfig): Queue.Queue<T> {
  const queue = new Queue<T>(config.name, {
    createClient: (type) => {
      switch (type) {
        case "client":
          return config.redis.duplicate();
        case "subscriber":
          return config.redis.duplicate();
        case "bclient":
          return config.redis.duplicate();
        default:
          return config.redis.duplicate();
      }
    },
    defaultJobOptions: {
      removeOnComplete: 100, // Keep last 100 completed jobs
      removeOnFail: 1000, // Keep last 1000 failed jobs
      attempts: 3,
      backoff: {
        type: "exponential",
        delay: 2000,
      },
      ...config.defaultJobOptions,
    },
  });

  // Global error handler
  queue.on("error", (error) => {
    console.error(`Queue ${config.name} error:`, error);
  });

  return queue;
}

// Email queue with typed processor
const emailQueue = createQueue<EmailJobData>({
  name: "email",
  redis: new Redis(process.env.REDIS_URL),
});

// Define processor
emailQueue.process(async (job: Job<EmailJobData>) => {
  const { to, subject, template, context } = job.data;

  // Update progress
  await job.progress(10);

  // Render template
  const html = await renderTemplate(template, context);
  await job.progress(50);

  // Send email
  await emailService.send({ to, subject, html });
  await job.progress(100);

  return { sent: true, messageId: `msg_${Date.now()}` };
});

// Add job with options
async function sendEmail(
  data: EmailJobData,
  options?: JobOptions,
): Promise<Job<EmailJobData>> {
  return emailQueue.add(data, {
    priority: options?.priority || 0,
    delay: options?.delay || 0,
    jobId: options?.jobId, // For deduplication
    ...options,
  });
}

// Bulk job addition
async function sendBulkEmails(
  emails: EmailJobData[],
): Promise<Job<EmailJobData>[]> {
  const jobs = emails.map((data, index) => ({
    data,
    opts: {
      jobId: `bulk_${Date.now()}_${index}`,
    },
  }));

  return emailQueue.addBulk(jobs);
}
```

**Celery (Python)**:

```python
from celery import Celery, Task
from celery.exceptions import MaxRetriesExceededError
from typing import Any, Dict, Optional
import logging

# Celery configuration
app = Celery('tasks')
app.config_from_object({
    'broker_url': 'redis://localhost:6379/0',
    'result_backend': 'redis://localhost:6379/1',
    'task_serializer': 'json',
    'result_serializer': 'json',
    'accept_content': ['json'],
    'timezone': 'UTC',
    'task_track_started': True,
    'task_time_limit': 300,  # 5 minutes hard limit
    'task_soft_time_limit': 240,  # 4 minutes soft limit
    'worker_prefetch_multiplier': 4,
    'task_acks_late': True,  # Acknowledge after task completes
    'task_reject_on_worker_lost': True,
})

logger = logging.getLogger(__name__)

# Base task with retry logic
class BaseTask(Task):
    autoretry_for = (Exception,)
    retry_kwargs = {'max_retries': 3}
    retry_backoff = True
    retry_backoff_max = 600  # 10 minutes max
    retry_jitter = True

    def on_failure(self, exc, task_id, args, kwargs, einfo):
        logger.error(f'Task {self.name}[{task_id}] failed: {exc}')

    def on_retry(self, exc, task_id, args, kwargs, einfo):
        logger.warning(f'Task {self.name}[{task_id}] retrying: {exc}')

    def on_success(self, retval, task_id, args, kwargs):
        logger.info(f'Task {self.name}[{task_id}] succeeded')

# Email task
@app.task(base=BaseTask, bind=True, name='send_email')
def send_email(
    self,
    to: str,
    subject: str,
    template: str,
    context: Dict[str, Any]
) -> Dict[str, Any]:
    try:
        # Update state
        self.update_state(state='PROGRESS', meta={'progress': 10})

        # Render template
        html = render_template(template, context)
        self.update_state(state='PROGRESS', meta={'progress': 50})

        # Send email
        message_id = email_service.send(to=to, subject=subject, html=html)
        self.update_state(state='PROGRESS', meta={'progress': 100})

        return {'sent': True, 'message_id': message_id}

    except ConnectionError as exc:
        raise self.retry(exc=exc, countdown=60)

# Image processing with chaining
@app.task(base=BaseTask, bind=True, name='process_image')
def process_image(self, image_id: str, operations: list) -> Dict[str, Any]:
    image = load_image(image_id)

    for i, op in enumerate(operations):
        progress = int((i + 1) / len(operations) * 100)
        self.update_state(state='PROGRESS', meta={'progress': progress, 'operation': op['type']})

        if op['type'] == 'resize':
            image = resize_image(image, **op['params'])
        elif op['type'] == 'crop':
            image = crop_image(image, **op['params'])
        elif op['type'] == 'compress':
            image = compress_image(image, **op['params'])

    url = save_image(image, image_id)
    return {'url': url, 'operations_count': len(operations)}

# Task chaining example
from celery import chain, group, chord

def process_order(order_id: str):
    """Process order with chained tasks."""
    workflow = chain(
        validate_order.s(order_id),
        reserve_inventory.s(),
        process_payment.s(),
        send_confirmation.s(),
    )
    return workflow.apply_async()

def process_bulk_images(image_ids: list):
    """Process multiple images in parallel, then aggregate results."""
    workflow = chord(
        group(process_image.s(img_id, [{'type': 'resize', 'params': {'width': 800}}])
              for img_id in image_ids),
        aggregate_results.s()
    )
    return workflow.apply_async()
```

**Sidekiq (Ruby)**:

```ruby
# config/initializers/sidekiq.rb
Sidekiq.configure_server do |config|
  config.redis = { url: ENV['REDIS_URL'], network_timeout: 5 }
  config.death_handlers << ->(job, ex) do
    # Handle job failure
    ErrorReporter.report(ex, job: job)
  end
end

Sidekiq.configure_client do |config|
  config.redis = { url: ENV['REDIS_URL'], network_timeout: 5 }
end

# app/workers/email_worker.rb
class EmailWorker
  include Sidekiq::Worker

  sidekiq_options queue: :default,
                  retry: 5,
                  backtrace: true,
                  dead: true

  sidekiq_retry_in do |count, exception|
    # Exponential backoff: 1, 8, 27, 64, 125 seconds
    (count + 1) ** 3
  end

  sidekiq_retries_exhausted do |msg, exception|
    Rails.logger.error "Job #{msg['jid']} exhausted retries: #{exception.message}"
    DeadJobNotifier.notify(msg, exception)
  end

  def perform(to, subject, template, context)
    html = ApplicationController.render(
      template: template,
      locals: context.symbolize_keys
    )

    EmailService.send(to: to, subject: subject, html: html)
  end
end

# app/workers/batch_worker.rb
class BatchWorker
  include Sidekiq::Worker

  def perform(batch_id)
    batch = Batch.find(batch_id)

    batch.items.find_each do |item|
      ItemProcessor.perform_async(item.id)
    end
  end
end

# Using Sidekiq Batches (Pro feature)
class ImportWorker
  include Sidekiq::Worker

  def perform(import_id)
    import = Import.find(import_id)

    batch = Sidekiq::Batch.new
    batch.description = "Import #{import_id}"
    batch.on(:complete, ImportCallbacks, import_id: import_id)

    batch.jobs do
      import.rows.each_with_index do |row, index|
        ImportRowWorker.perform_async(import_id, index, row)
      end
    end
  end
end

class ImportCallbacks
  def on_complete(status, options)
    import = Import.find(options['import_id'])

    if status.failures.zero?
      import.update!(status: 'completed')
    else
      import.update!(status: 'completed_with_errors', error_count: status.failures)
    end
  end
end
```

### Scheduled Jobs and Cron Patterns

```typescript
// Bull scheduler
import Queue from "bull";

const scheduledQueue = new Queue("scheduled-tasks", process.env.REDIS_URL);

// Repeatable jobs
async function setupScheduledJobs(): Promise<void> {
  // Clean up every hour
  await scheduledQueue.add(
    "cleanup",
    {},
    {
      repeat: { cron: "0 * * * *" }, // Every hour
      jobId: "cleanup-hourly",
    },
  );

  // Daily report at 9 AM
  await scheduledQueue.add(
    "daily-report",
    {},
    {
      repeat: { cron: "0 9 * * *" },
      jobId: "daily-report",
    },
  );

  // Every 5 minutes
  await scheduledQueue.add(
    "health-check",
    {},
    {
      repeat: { every: 5 * 60 * 1000 }, // 5 minutes in ms
      jobId: "health-check",
    },
  );

  // Weekly on Sunday at midnight
  await scheduledQueue.add(
    "weekly-cleanup",
    {},
    {
      repeat: { cron: "0 0 * * 0" },
      jobId: "weekly-cleanup",
    },
  );
}

// Process scheduled jobs
scheduledQueue.process("cleanup", async (job) => {
  await cleanupOldRecords();
  return { cleaned: true };
});

scheduledQueue.process("daily-report", async (job) => {
  const report = await generateDailyReport();
  await sendReportEmail(report);
  return { reportId: report.id };
});

// List scheduled jobs
async function getScheduledJobs(): Promise<
  Array<{ name: string; next: Date; cron: string }>
> {
  const repeatableJobs = await scheduledQueue.getRepeatableJobs();

  return repeatableJobs.map((job) => ({
    name: job.name,
    next: new Date(job.next),
    cron: job.cron || `Every ${job.every}ms`,
  }));
}

// Remove scheduled job
async function removeScheduledJob(jobId: string): Promise<void> {
  const jobs = await scheduledQueue.getRepeatableJobs();
  const job = jobs.find((j) => j.id === jobId);

  if (job) {
    await scheduledQueue.removeRepeatableByKey(job.key);
  }
}
```

```python
# Celery Beat scheduler
from celery import Celery
from celery.schedules import crontab

app = Celery('tasks')

app.conf.beat_schedule = {
    # Every hour
    'cleanup-hourly': {
        'task': 'tasks.cleanup',
        'schedule': crontab(minute=0),  # Every hour at minute 0
    },

    # Daily at 9 AM
    'daily-report': {
        'task': 'tasks.daily_report',
        'schedule': crontab(hour=9, minute=0),
    },

    # Every 5 minutes
    'health-check': {
        'task': 'tasks.health_check',
        'schedule': 300.0,  # 5 minutes in seconds
    },

    # Weekly on Sunday at midnight
    'weekly-cleanup': {
        'task': 'tasks.weekly_cleanup',
        'schedule': crontab(hour=0, minute=0, day_of_week=0),
    },

    # First day of month at 6 AM
    'monthly-report': {
        'task': 'tasks.monthly_report',
        'schedule': crontab(hour=6, minute=0, day_of_month=1),
    },

    # With arguments
    'check-expiring-subscriptions': {
        'task': 'tasks.check_subscriptions',
        'schedule': crontab(hour=8, minute=0),
        'args': ('expiring',),
        'kwargs': {'days_ahead': 7},
    },
}

# Dynamic schedule with database
from django_celery_beat.models import PeriodicTask, CrontabSchedule
import json

def create_scheduled_task(name: str, task: str, cron: str, args: list = None, kwargs: dict = None):
    """Create a scheduled task dynamically."""
    # Parse cron expression
    minute, hour, day_of_month, month, day_of_week = cron.split()

    schedule, _ = CrontabSchedule.objects.get_or_create(
        minute=minute,
        hour=hour,
        day_of_month=day_of_month,
        month_of_year=month,
        day_of_week=day_of_week,
    )

    PeriodicTask.objects.update_or_create(
        name=name,
        defaults={
            'task': task,
            'crontab': schedule,
            'args': json.dumps(args or []),
            'kwargs': json.dumps(kwargs or {}),
            'enabled': True,
        },
    )
```

### Worker Pool Management

```typescript
import Queue, { Job } from "bull";
import os from "os";

interface WorkerPoolConfig {
  concurrency: number;
  limiter?: {
    max: number;
    duration: number;
  };
}

class WorkerPool {
  private queues: Map<string, Queue.Queue> = new Map();
  private isShuttingDown = false;

  constructor(private config: WorkerPoolConfig) {
    // Graceful shutdown
    process.on("SIGTERM", () => this.shutdown());
    process.on("SIGINT", () => this.shutdown());
  }

  registerQueue<T>(
    name: string,
    processor: (job: Job<T>) => Promise<unknown>,
  ): Queue.Queue<T> {
    const queue = new Queue<T>(name, process.env.REDIS_URL!, {
      limiter: this.config.limiter,
    });

    // Process with concurrency
    queue.process(this.config.concurrency, async (job: Job<T>) => {
      if (this.isShuttingDown) {
        throw new Error("Worker shutting down");
      }
      return processor(job);
    });

    // Event handlers
    queue.on("completed", (job, result) => {
      console.log(`Job ${job.id} completed:`, result);
    });

    queue.on("failed", (job, err) => {
      console.error(`Job ${job?.id} failed:`, err);
    });

    queue.on("stalled", (job) => {
      console.warn(`Job ${job} stalled`);
    });

    this.queues.set(name, queue);
    return queue;
  }

  async shutdown(): Promise<void> {
    console.log("Initiating graceful shutdown...");
    this.isShuttingDown = true;

    // Stop accepting new jobs
    const closePromises = Array.from(this.queues.values()).map(
      async (queue) => {
        await queue.pause(true); // Pause and wait for active jobs
        await queue.close();
      },
    );

    await Promise.all(closePromises);
    console.log("All queues closed");
    process.exit(0);
  }

  async getStats(): Promise<Record<string, QueueStats>> {
    const stats: Record<string, QueueStats> = {};

    for (const [name, queue] of this.queues) {
      const [waiting, active, completed, failed, delayed] = await Promise.all([
        queue.getWaitingCount(),
        queue.getActiveCount(),
        queue.getCompletedCount(),
        queue.getFailedCount(),
        queue.getDelayedCount(),
      ]);

      stats[name] = { waiting, active, completed, failed, delayed };
    }

    return stats;
  }
}

interface QueueStats {
  waiting: number;
  active: number;
  completed: number;
  failed: number;
  delayed: number;
}

// Usage
const pool = new WorkerPool({
  concurrency: os.cpus().length,
  limiter: {
    max: 100, // Max 100 jobs
    duration: 1000, // Per second
  },
});

pool.registerQueue<EmailJobData>("email", async (job) => {
  await sendEmail(job.data);
});

pool.registerQueue<ImageProcessingJobData>("images", async (job) => {
  await processImage(job.data);
});
```

```python
# Celery worker management
from celery import Celery
from celery.signals import worker_process_init, worker_shutdown
import multiprocessing

app = Celery('tasks')

# Worker configuration
app.conf.update(
    worker_concurrency=multiprocessing.cpu_count(),
    worker_prefetch_multiplier=2,  # Prefetch 2 tasks per worker
    worker_max_tasks_per_child=1000,  # Restart worker after 1000 tasks
    worker_max_memory_per_child=200000,  # 200MB limit
    task_acks_late=True,
    task_reject_on_worker_lost=True,
)

# Per-worker initialization
@worker_process_init.connect
def init_worker(**kwargs):
    """Initialize resources for each worker process."""
    # Initialize database connection pool
    db.connect()
    # Warm up caches
    cache.warm_up()

@worker_shutdown.connect
def cleanup_worker(**kwargs):
    """Clean up resources on worker shutdown."""
    db.close()
    cache.flush()

# Task routing for specialized workers
app.conf.task_routes = {
    'tasks.send_email': {'queue': 'email'},
    'tasks.process_image': {'queue': 'images'},
    'tasks.heavy_computation': {'queue': 'compute'},
    'tasks.*': {'queue': 'default'},
}

# Queue-specific worker command:
# celery -A tasks worker -Q email --concurrency=4
# celery -A tasks worker -Q images --concurrency=2
# celery -A tasks worker -Q compute --concurrency=1

# Auto-scaling with Celery
app.conf.worker_autoscaler = 'celery.worker.autoscale:Autoscaler'
app.conf.worker_autoscale_max = 10
app.conf.worker_autoscale_min = 2
```

### Job Priorities and Fairness

```typescript
// Priority queues with Bull
interface PriorityJobData {
  type: string;
  payload: unknown;
  priority: "critical" | "high" | "normal" | "low";
}

const priorityMap = {
  critical: 1, // Highest priority (processed first)
  high: 5,
  normal: 10,
  low: 20,
};

async function addPriorityJob(
  data: PriorityJobData,
): Promise<Job<PriorityJobData>> {
  return queue.add(data, {
    priority: priorityMap[data.priority],
    // Critical jobs don't wait
    delay: data.priority === "critical" ? 0 : undefined,
  });
}

// Fair scheduling with multiple queues
class FairScheduler {
  private queues: Map<string, Queue.Queue> = new Map();
  private weights: Map<string, number> = new Map();

  constructor(queueConfigs: Array<{ name: string; weight: number }>) {
    for (const config of queueConfigs) {
      const queue = new Queue(config.name, process.env.REDIS_URL!);
      this.queues.set(config.name, queue);
      this.weights.set(config.name, config.weight);
    }
  }

  // Weighted round-robin processing
  async process(
    handler: (queueName: string, job: Job) => Promise<void>,
  ): Promise<void> {
    const totalWeight = Array.from(this.weights.values()).reduce(
      (a, b) => a + b,
      0,
    );

    for (const [name, queue] of this.queues) {
      const weight = this.weights.get(name)!;
      const concurrency = Math.max(1, Math.floor((weight / totalWeight) * 10));

      queue.process(concurrency, async (job) => {
        await handler(name, job);
      });
    }
  }
}

// Usage: Process premium customers first
const scheduler = new FairScheduler([
  { name: "premium", weight: 5 }, // 50% of capacity
  { name: "standard", weight: 3 }, // 30% of capacity
  { name: "free", weight: 2 }, // 20% of capacity
]);

await scheduler.process(async (queueName, job) => {
  console.log(`Processing ${queueName} job:`, job.id);
  await processJob(job);
});
```

### Idempotency and Retry Strategies

```typescript
import Queue, { Job, JobOptions } from "bull";
import { createHash } from "crypto";

// Idempotency key generation
function generateIdempotencyKey(data: unknown): string {
  const hash = createHash("sha256");
  hash.update(JSON.stringify(data));
  return hash.digest("hex");
}

// Idempotent job processor
class IdempotentProcessor<T> {
  private processedKeys: Set<string> = new Set();
  private redis: Redis;

  constructor(
    private queue: Queue.Queue<T>,
    redis: Redis,
  ) {
    this.redis = redis;
  }

  async process(handler: (job: Job<T>) => Promise<unknown>): Promise<void> {
    this.queue.process(async (job: Job<T>) => {
      const idempotencyKey = job.opts.jobId || generateIdempotencyKey(job.data);

      // Check if already processed
      const existing = await this.redis.get(`processed:${idempotencyKey}`);
      if (existing) {
        console.log(`Job ${job.id} already processed, skipping`);
        return JSON.parse(existing);
      }

      // Process job
      const result = await handler(job);

      // Mark as processed with TTL
      await this.redis.setex(
        `processed:${idempotencyKey}`,
        86400, // 24 hours
        JSON.stringify(result),
      );

      return result;
    });
  }
}

// Custom retry strategies
interface RetryStrategy {
  type: "exponential" | "linear" | "fixed" | "custom";
  baseDelay: number;
  maxDelay?: number;
  maxRetries: number;
  jitter?: boolean;
  retryOn?: (error: Error) => boolean;
}

function calculateDelay(strategy: RetryStrategy, attempt: number): number {
  let delay: number;

  switch (strategy.type) {
    case "exponential":
      delay = strategy.baseDelay * Math.pow(2, attempt - 1);
      break;
    case "linear":
      delay = strategy.baseDelay * attempt;
      break;
    case "fixed":
      delay = strategy.baseDelay;
      break;
    default:
      delay = strategy.baseDelay;
  }

  // Apply max delay cap
  if (strategy.maxDelay) {
    delay = Math.min(delay, strategy.maxDelay);
  }

  // Add jitter (up to 20% variation)
  if (strategy.jitter) {
    const jitterFactor = 0.8 + Math.random() * 0.4; // 0.8 to 1.2
    delay = Math.floor(delay * jitterFactor);
  }

  return delay;
}

// Retry with dead letter queue
class RetryableQueue<T> {
  private mainQueue: Queue.Queue<T>;
  private dlq: Queue.Queue<T>;
  private strategy: RetryStrategy;

  constructor(name: string, strategy: RetryStrategy) {
    this.mainQueue = new Queue<T>(name, process.env.REDIS_URL!);
    this.dlq = new Queue<T>(`${name}-dlq`, process.env.REDIS_URL!);
    this.strategy = strategy;
  }

  async process(handler: (job: Job<T>) => Promise<unknown>): Promise<void> {
    this.mainQueue.process(async (job: Job<T>) => {
      const attempts = job.attemptsMade;

      try {
        return await handler(job);
      } catch (error) {
        const err = error as Error;

        // Check if error is retryable
        if (this.strategy.retryOn && !this.strategy.retryOn(err)) {
          await this.moveToDLQ(job, err);
          throw err;
        }

        // Check max retries
        if (attempts >= this.strategy.maxRetries) {
          await this.moveToDLQ(job, err);
          throw err;
        }

        // Retry with calculated delay
        const delay = calculateDelay(this.strategy, attempts + 1);
        throw new Error(`Retry in ${delay}ms: ${err.message}`);
      }
    });
  }

  private async moveToDLQ(job: Job<T>, error: Error): Promise<void> {
    await this.dlq.add({
      originalJob: job.data,
      error: error.message,
      failedAt: new Date().toISOString(),
      attempts: job.attemptsMade,
    } as unknown as T);
  }

  async retryFromDLQ(jobId: string): Promise<void> {
    const job = await this.dlq.getJob(jobId);
    if (!job) return;

    const dlqData = job.data as unknown as { originalJob: T };
    await this.mainQueue.add(dlqData.originalJob);
    await job.remove();
  }
}
```

### Job Monitoring and Dead Jobs

```typescript
import Queue, { Job, JobCounts, JobStatus } from "bull";
import { EventEmitter } from "events";

interface JobMetrics {
  queue: string;
  counts: JobCounts;
  latency: {
    avg: number;
    p50: number;
    p95: number;
    p99: number;
  };
  throughput: number; // jobs per minute
  errorRate: number;
}

class JobMonitor extends EventEmitter {
  private queues: Queue.Queue[] = [];
  private metricsHistory: Map<string, number[]> = new Map();

  addQueue(queue: Queue.Queue): void {
    this.queues.push(queue);

    queue.on("completed", (job, result) => {
      this.recordMetric(queue.name, "completed", job);
      this.emit("job:completed", { queue: queue.name, job, result });
    });

    queue.on("failed", (job, err) => {
      this.recordMetric(queue.name, "failed", job!);
      this.emit("job:failed", { queue: queue.name, job, error: err });

      // Alert on high failure rate
      this.checkErrorRate(queue.name);
    });

    queue.on("stalled", (job) => {
      this.emit("job:stalled", { queue: queue.name, jobId: job });
    });
  }

  private recordMetric(queueName: string, type: string, job: Job): void {
    const duration = Date.now() - job.timestamp;
    const key = `${queueName}:${type}:duration`;

    const history = this.metricsHistory.get(key) || [];
    history.push(duration);

    // Keep last 1000 samples
    if (history.length > 1000) {
      history.shift();
    }

    this.metricsHistory.set(key, history);
  }

  private checkErrorRate(queueName: string): void {
    const completed =
      this.metricsHistory.get(`${queueName}:completed:duration`)?.length || 0;
    const failed =
      this.metricsHistory.get(`${queueName}:failed:duration`)?.length || 0;

    if (completed + failed > 10) {
      const errorRate = failed / (completed + failed);
      if (errorRate > 0.1) {
        // > 10% error rate
        this.emit("alert:high_error_rate", { queue: queueName, errorRate });
      }
    }
  }

  async getMetrics(queueName: string): Promise<JobMetrics> {
    const queue = this.queues.find((q) => q.name === queueName);
    if (!queue) throw new Error(`Queue ${queueName} not found`);

    const counts = await queue.getJobCounts();
    const durations =
      this.metricsHistory.get(`${queueName}:completed:duration`) || [];

    return {
      queue: queueName,
      counts,
      latency: this.calculateLatencyPercentiles(durations),
      throughput: this.calculateThroughput(durations),
      errorRate: this.calculateErrorRate(queueName),
    };
  }

  private calculateLatencyPercentiles(
    durations: number[],
  ): JobMetrics["latency"] {
    if (durations.length === 0) {
      return { avg: 0, p50: 0, p95: 0, p99: 0 };
    }

    const sorted = [...durations].sort((a, b) => a - b);
    const avg = sorted.reduce((a, b) => a + b, 0) / sorted.length;

    return {
      avg: Math.round(avg),
      p50: sorted[Math.floor(sorted.length * 0.5)],
      p95: sorted[Math.floor(sorted.length * 0.95)],
      p99: sorted[Math.floor(sorted.length * 0.99)],
    };
  }

  private calculateThroughput(durations: number[]): number {
    // Jobs completed in last minute
    const oneMinuteAgo = Date.now() - 60000;
    const recentJobs = durations.filter((_, i) => i > durations.length - 100);
    return recentJobs.length;
  }

  private calculateErrorRate(queueName: string): number {
    const completed =
      this.metricsHistory.get(`${queueName}:completed:duration`)?.length || 0;
    const failed =
      this.metricsHistory.get(`${queueName}:failed:duration`)?.length || 0;
    const total = completed + failed;
    return total > 0 ? failed / total : 0;
  }

  // Dead job management
  async getDeadJobs(queueName: string, limit: number = 100): Promise<Job[]> {
    const queue = this.queues.find((q) => q.name === queueName);
    if (!queue) throw new Error(`Queue ${queueName} not found`);

    return queue.getFailed(0, limit);
  }

  async retryDeadJob(queueName: string, jobId: string): Promise<void> {
    const queue = this.queues.find((q) => q.name === queueName);
    if (!queue) throw new Error(`Queue ${queueName} not found`);

    const job = await queue.getJob(jobId);
    if (!job) throw new Error(`Job ${jobId} not found`);

    await job.retry();
  }

  async retryAllDeadJobs(queueName: string): Promise<number> {
    const deadJobs = await this.getDeadJobs(queueName);
    let retried = 0;

    for (const job of deadJobs) {
      try {
        await job.retry();
        retried++;
      } catch (error) {
        console.error(`Failed to retry job ${job.id}:`, error);
      }
    }

    return retried;
  }

  async cleanDeadJobs(
    queueName: string,
    olderThan: number = 86400000,
  ): Promise<number> {
    const queue = this.queues.find((q) => q.name === queueName);
    if (!queue) throw new Error(`Queue ${queueName} not found`);

    const cleaned = await queue.clean(olderThan, "failed");
    return cleaned.length;
  }
}

// Dashboard API endpoints
import express from "express";

function createMonitoringRouter(monitor: JobMonitor): express.Router {
  const router = express.Router();

  router.get("/queues/:name/metrics", async (req, res) => {
    try {
      const metrics = await monitor.getMetrics(req.params.name);
      res.json(metrics);
    } catch (error) {
      res.status(404).json({ error: (error as Error).message });
    }
  });

  router.get("/queues/:name/dead", async (req, res) => {
    const limit = parseInt(req.query.limit as string) || 100;
    const jobs = await monitor.getDeadJobs(req.params.name, limit);
    res.json(
      jobs.map((j) => ({
        id: j.id,
        data: j.data,
        failedReason: j.failedReason,
        attemptsMade: j.attemptsMade,
        timestamp: j.timestamp,
      })),
    );
  });

  router.post("/queues/:name/dead/:jobId/retry", async (req, res) => {
    try {
      await monitor.retryDeadJob(req.params.name, req.params.jobId);
      res.json({ success: true });
    } catch (error) {
      res.status(400).json({ error: (error as Error).message });
    }
  });

  router.post("/queues/:name/dead/retry-all", async (req, res) => {
    const retried = await monitor.retryAllDeadJobs(req.params.name);
    res.json({ retried });
  });

  router.delete("/queues/:name/dead", async (req, res) => {
    const olderThan = parseInt(req.query.olderThan as string) || 86400000;
    const cleaned = await monitor.cleanDeadJobs(req.params.name, olderThan);
    res.json({ cleaned });
  });

  return router;
}
```

## Best Practices

1. **Idempotency**
   - Design jobs to be safely re-executed
   - Use unique job IDs for deduplication
   - Store processed state externally

2. **Retry Strategies**
   - Use exponential backoff with jitter
   - Set maximum retry limits
   - Distinguish between retryable and non-retryable errors

3. **Monitoring**
   - Track queue depths and processing latency
   - Alert on high error rates or growing queues
   - Monitor worker health and memory usage

4. **Graceful Shutdown**
   - Complete in-progress jobs before shutdown
   - Use signals (SIGTERM, SIGINT) properly
   - Set reasonable timeouts for job completion

5. **Resource Management**
   - Set appropriate concurrency limits
   - Use worker pools for CPU-bound tasks
   - Implement rate limiting for external APIs

## Examples

### Complete Worker Service

```typescript
import Queue, { Job } from "bull";
import { Redis } from "ioredis";

interface WorkerConfig {
  queues: Array<{
    name: string;
    concurrency: number;
    processor: (job: Job) => Promise<unknown>;
  }>;
  redis: Redis;
  shutdownTimeout: number;
}

class WorkerService {
  private queues: Map<string, Queue.Queue> = new Map();
  private isShuttingDown = false;
  private activeJobs = 0;

  constructor(private config: WorkerConfig) {}

  async start(): Promise<void> {
    // Setup queues
    for (const queueConfig of this.config.queues) {
      const queue = new Queue(queueConfig.name, {
        createClient: () => this.config.redis.duplicate(),
      });

      queue.process(queueConfig.concurrency, async (job) => {
        if (this.isShuttingDown) {
          throw new Error("Worker shutting down");
        }

        this.activeJobs++;
        try {
          return await queueConfig.processor(job);
        } finally {
          this.activeJobs--;
        }
      });

      this.queues.set(queueConfig.name, queue);
    }

    // Setup graceful shutdown
    process.on("SIGTERM", () => this.shutdown());
    process.on("SIGINT", () => this.shutdown());

    console.log("Worker service started");
  }

  private async shutdown(): Promise<void> {
    if (this.isShuttingDown) return;
    this.isShuttingDown = true;

    console.log("Shutting down worker service...");

    // Pause all queues
    await Promise.all(
      Array.from(this.queues.values()).map((q) => q.pause(true)),
    );

    // Wait for active jobs to complete
    const startTime = Date.now();
    while (
      this.activeJobs > 0 &&
      Date.now() - startTime < this.config.shutdownTimeout
    ) {
      await new Promise((r) => setTimeout(r, 100));
    }

    if (this.activeJobs > 0) {
      console.warn(`Forcing shutdown with ${this.activeJobs} active jobs`);
    }

    // Close all queues
    await Promise.all(Array.from(this.queues.values()).map((q) => q.close()));

    console.log("Worker service stopped");
    process.exit(0);
  }
}

// Usage
const worker = new WorkerService({
  redis: new Redis(process.env.REDIS_URL),
  shutdownTimeout: 30000,
  queues: [
    {
      name: "email",
      concurrency: 5,
      processor: async (job) => {
        await sendEmail(job.data);
      },
    },
    {
      name: "images",
      concurrency: 2,
      processor: async (job) => {
        await processImage(job.data);
      },
    },
  ],
});

worker.start();
```
