---
name: kubernetes
description: Designs and manages Kubernetes deployments, services, configurations, and cluster resources. Trigger keywords: kubernetes, k8s, deployment, pod, service, helm, kubectl, cluster, ingress.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# Kubernetes

## Overview

This skill covers Kubernetes resource configuration, deployment strategies, and cluster management. It helps create production-ready Kubernetes manifests and troubleshoot cluster issues.

## Instructions

### 1. Design Resource Architecture

- Plan namespace organization
- Define resource requests/limits
- Design service mesh topology
- Plan for high availability

### 2. Create Resource Manifests

- Write Deployments/StatefulSets
- Configure Services and Ingress
- Set up ConfigMaps and Secrets
- Define RBAC policies

### 3. Implement Best Practices

- Use resource quotas
- Configure health probes
- Set up pod disruption budgets
- Implement network policies

### 4. Monitor and Debug

- Check pod logs and events
- Analyze resource usage
- Debug networking issues
- Handle failed deployments

## Best Practices

1. **Use Namespaces**: Organize resources logically
2. **Set Resource Limits**: Prevent resource exhaustion
3. **Health Probes**: Configure liveness and readiness
4. **Rolling Updates**: Zero-downtime deployments
5. **Secrets Management**: Never hardcode secrets
6. **Label Everything**: Enable filtering and selection
7. **Use Helm/Kustomize**: Template and manage manifests

## Examples

### Example 1: Production Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-server
  namespace: production
  labels:
    app: api-server
    version: v1.2.0
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: api-server
  template:
    metadata:
      labels:
        app: api-server
        version: v1.2.0
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
    spec:
      serviceAccountName: api-server
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000

      containers:
        - name: api
          image: myregistry.io/api-server:v1.2.0
          imagePullPolicy: IfNotPresent

          ports:
            - name: http
              containerPort: 8080
              protocol: TCP

          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: api-secrets
                  key: database-url
            - name: LOG_LEVEL
              valueFrom:
                configMapKeyRef:
                  name: api-config
                  key: log-level

          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi

          livenessProbe:
            httpGet:
              path: /health/live
              port: http
            initialDelaySeconds: 15
            periodSeconds: 20
            timeoutSeconds: 5
            failureThreshold: 3

          readinessProbe:
            httpGet:
              path: /health/ready
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
            timeoutSeconds: 3
            failureThreshold: 3

          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            capabilities:
              drop:
                - ALL

          volumeMounts:
            - name: tmp
              mountPath: /tmp
            - name: cache
              mountPath: /app/cache

      volumes:
        - name: tmp
          emptyDir: {}
        - name: cache
          emptyDir:
            sizeLimit: 100Mi

      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchLabels:
                    app: api-server
                topologyKey: kubernetes.io/hostname

      topologySpreadConstraints:
        - maxSkew: 1
          topologyKey: topology.kubernetes.io/zone
          whenUnsatisfiable: ScheduleAnyway
          labelSelector:
            matchLabels:
              app: api-server
```

### Example 2: Service and Ingress

```yaml
apiVersion: v1
kind: Service
metadata:
  name: api-server
  namespace: production
  labels:
    app: api-server
spec:
  type: ClusterIP
  ports:
    - port: 80
      targetPort: http
      protocol: TCP
      name: http
  selector:
    app: api-server

---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: api-server
  namespace: production
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
    nginx.ingress.kubernetes.io/rate-limit-window: "1m"
spec:
  tls:
    - hosts:
        - api.example.com
      secretName: api-tls-cert
  rules:
    - host: api.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: api-server
                port:
                  number: 80
```

### Example 3: ConfigMap and Secret

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: api-config
  namespace: production
data:
  log-level: "info"
  max-connections: "100"
  cache-ttl: "3600"
  feature-flags: |
    {
      "new-checkout": true,
      "beta-features": false
    }

---
apiVersion: v1
kind: Secret
metadata:
  name: api-secrets
  namespace: production
type: Opaque
stringData:
  database-url: "postgresql://user:password@db-host:5432/myapp"
  api-key: "super-secret-key"
```

### Example 4: Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: api-server
  namespace: production
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: api-server
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
        - type: Percent
          value: 10
          periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
        - type: Percent
          value: 100
          periodSeconds: 15
        - type: Pods
          value: 4
          periodSeconds: 15
      selectPolicy: Max
```
