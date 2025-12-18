---
name: docker
description: Creates and optimizes Docker configurations including Dockerfiles, docker-compose files, and container orchestration. Trigger keywords: docker, dockerfile, container, image, docker-compose, containerize, build image.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# Docker

## Overview

This skill focuses on containerization with Docker, including writing efficient Dockerfiles, composing multi-container applications, and following security and performance best practices.

## Instructions

### 1. Analyze Application Requirements

- Identify runtime dependencies
- Determine build vs runtime needs
- Plan for configuration management
- Consider data persistence needs

### 2. Write Efficient Dockerfiles

- Choose appropriate base images
- Optimize layer caching
- Minimize image size
- Handle secrets properly

### 3. Configure Compose Files

- Define service dependencies
- Set up networking
- Configure volumes for persistence
- Manage environment variables

### 4. Security Hardening

- Use non-root users
- Scan for vulnerabilities
- Minimize attack surface
- Keep images updated

## Best Practices

1. **Use Official Base Images**: Start from trusted sources
2. **Multi-Stage Builds**: Separate build and runtime environments
3. **Minimize Layers**: Combine related commands
4. **Don't Run as Root**: Create and use non-root users
5. **Use .dockerignore**: Exclude unnecessary files
6. **Pin Versions**: Use specific tags, not `latest`
7. **Health Checks**: Add container health monitoring

## Examples

### Example 1: Multi-Stage Python Dockerfile

```dockerfile
# Build stage
FROM python:3.12-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
COPY requirements.txt .
RUN pip wheel --no-cache-dir --no-deps --wheel-dir /app/wheels -r requirements.txt

# Runtime stage
FROM python:3.12-slim AS runtime

# Create non-root user
RUN groupadd --gid 1000 appgroup && \
    useradd --uid 1000 --gid appgroup --shell /bin/bash --create-home appuser

WORKDIR /app

# Copy wheels from builder
COPY --from=builder /app/wheels /wheels
RUN pip install --no-cache-dir /wheels/* && rm -rf /wheels

# Copy application code
COPY --chown=appuser:appgroup . .

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run application
CMD ["gunicorn", "--bind", "0.0.0.0:8000", "--workers", "4", "app:create_app()"]
```

### Example 2: Node.js Dockerfile with Security

```dockerfile
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci --only=production

# Copy source code
COPY . .

# Build application
RUN npm run build

# Production stage
FROM node:20-alpine AS production

# Add security updates
RUN apk update && apk upgrade && rm -rf /var/cache/apk/*

# Create non-root user
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nextjs -u 1001

WORKDIR /app

# Copy built assets
COPY --from=builder --chown=nextjs:nodejs /app/dist ./dist
COPY --from=builder --chown=nextjs:nodejs /app/node_modules ./node_modules
COPY --from=builder --chown=nextjs:nodejs /app/package.json ./

USER nextjs

EXPOSE 3000

ENV NODE_ENV=production

CMD ["node", "dist/server.js"]
```

### Example 3: Docker Compose for Development

```yaml
version: "3.8"

services:
  app:
    build:
      context: .
      dockerfile: Dockerfile.dev
    ports:
      - "3000:3000"
    volumes:
      - .:/app
      - /app/node_modules
    environment:
      - NODE_ENV=development
      - DATABASE_URL=postgres://user:pass@db:5432/myapp
      - REDIS_URL=redis://cache:6379
    depends_on:
      db:
        condition: service_healthy
      cache:
        condition: service_started
    networks:
      - app-network

  db:
    image: postgres:16-alpine
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    environment:
      POSTGRES_USER: user
      POSTGRES_PASSWORD: pass
      POSTGRES_DB: myapp
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U user -d myapp"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - app-network

  cache:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
    networks:
      - app-network

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./certs:/etc/nginx/certs:ro
    depends_on:
      - app
    networks:
      - app-network

volumes:
  postgres_data:
  redis_data:

networks:
  app-network:
    driver: bridge
```

### Example 4: .dockerignore

```
# Git
.git
.gitignore

# Dependencies
node_modules
__pycache__
*.pyc
.venv
venv

# Build artifacts
dist
build
*.egg-info

# IDE
.idea
.vscode
*.swp

# Testing
coverage
.pytest_cache
.nyc_output

# Environment files
.env
.env.local
.env.*.local

# Documentation
docs
*.md
!README.md

# Docker
Dockerfile*
docker-compose*
.docker

# Misc
.DS_Store
*.log
tmp
```
