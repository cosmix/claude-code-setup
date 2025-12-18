---
name: security-audit
description: Performs comprehensive security audits identifying vulnerabilities, misconfigurations, and security best practice violations. Trigger keywords: security, audit, vulnerability, CVE, OWASP, penetration, security review, hardening.
allowed-tools: Read, Grep, Glob, Bash
---

# Security Audit

## Overview

This skill provides comprehensive security auditing capabilities to identify vulnerabilities, misconfigurations, and security best practice violations in code, configurations, and infrastructure.

## Instructions

### 1. Scope Assessment

- Identify assets to audit
- Determine compliance requirements
- Review security policies
- Plan audit methodology

### 2. Code Security Review

- Search for hardcoded secrets
- Identify injection vulnerabilities
- Check authentication/authorization
- Review cryptographic implementations

### 3. Configuration Review

- Check access controls
- Review network configurations
- Audit service configurations
- Verify encryption settings

### 4. Report Findings

- Categorize by severity
- Provide remediation steps
- Prioritize fixes
- Document evidence

## Best Practices

1. **Defense in Depth**: Multiple security layers
2. **Least Privilege**: Minimum necessary permissions
3. **Secure Defaults**: Safe out-of-the-box settings
4. **Input Validation**: Never trust user input
5. **Encryption**: Protect data at rest and in transit
6. **Logging**: Comprehensive audit trails
7. **Updates**: Keep dependencies current

## Examples

### Example 1: Common Vulnerability Patterns

```python
# CRITICAL: SQL Injection
# Vulnerable
query = f"SELECT * FROM users WHERE id = {user_id}"
cursor.execute(query)

# Secure
query = "SELECT * FROM users WHERE id = %s"
cursor.execute(query, (user_id,))

# CRITICAL: Command Injection
# Vulnerable
os.system(f"ping {hostname}")

# Secure
import shlex
subprocess.run(["ping", shlex.quote(hostname)])

# HIGH: Cross-Site Scripting (XSS)
# Vulnerable
return f"<h1>Welcome {username}</h1>"

# Secure
from markupsafe import escape
return f"<h1>Welcome {escape(username)}</h1>"

# HIGH: Path Traversal
# Vulnerable
file_path = f"/uploads/{filename}"
with open(file_path) as f:
    return f.read()

# Secure
import os
base_dir = "/uploads"
safe_path = os.path.normpath(os.path.join(base_dir, filename))
if not safe_path.startswith(base_dir):
    raise SecurityError("Path traversal detected")

# MEDIUM: Insecure Deserialization
# Vulnerable
import pickle
data = pickle.loads(user_input)

# Secure
import json
data = json.loads(user_input)

# MEDIUM: Hardcoded Secrets
# Vulnerable
API_KEY = "sk-1234567890abcdef"

# Secure
API_KEY = os.environ.get("API_KEY")
```

### Example 2: Security Checklist

```markdown
## Authentication & Authorization

- [ ] Passwords hashed with bcrypt/argon2 (cost factor >= 10)
- [ ] MFA available for sensitive operations
- [ ] Session tokens are cryptographically random
- [ ] Session invalidation on logout
- [ ] Rate limiting on login attempts
- [ ] Account lockout after failed attempts

## Input Validation

- [ ] All inputs validated server-side
- [ ] Parameterized queries for all database operations
- [ ] Output encoding for HTML contexts
- [ ] File upload validation (type, size, content)
- [ ] URL validation and sanitization

## Cryptography

- [ ] TLS 1.2+ enforced for all connections
- [ ] Strong cipher suites only
- [ ] Certificates from trusted CAs
- [ ] Secrets stored in secure vault
- [ ] No deprecated algorithms (MD5, SHA1, DES)

## Access Control

- [ ] Principle of least privilege applied
- [ ] RBAC/ABAC properly implemented
- [ ] Resource authorization checked on every request
- [ ] Admin interfaces protected and audited

## Data Protection

- [ ] Sensitive data encrypted at rest
- [ ] PII handling compliant with regulations
- [ ] Data retention policies implemented
- [ ] Secure data deletion procedures

## Logging & Monitoring

- [ ] Security events logged
- [ ] Logs protected from tampering
- [ ] Alerting on suspicious activities
- [ ] Log retention meets compliance
```

### Example 3: Security Scanning Commands

```bash
# Secret scanning with trufflehog
trufflehog filesystem --directory=. --only-verified

# Dependency vulnerability scanning
npm audit --production
pip-audit
cargo audit

# Static analysis
semgrep --config=auto .
bandit -r src/

# Container scanning
trivy image myapp:latest
grype myapp:latest

# Infrastructure scanning
checkov -d terraform/
tfsec terraform/

# OWASP ZAP API scan
zap-api-scan.py -t https://api.example.com/openapi.json -f openapi

# SSL/TLS testing
testssl.sh https://example.com

# Kubernetes security
kubesec scan deployment.yaml
kube-bench run --targets=node
```

### Example 4: Security Headers Configuration

```nginx
# Nginx security headers
add_header X-Frame-Options "DENY" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';" always;
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
add_header Permissions-Policy "geolocation=(), microphone=(), camera=()" always;

# Hide server version
server_tokens off;
```
