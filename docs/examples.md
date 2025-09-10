# Real-World Blizz Workflows

This page shows practical examples of how developers and teams are using Blizz in their daily workflows.

## Individual Developer Workflows

### Daily Development Session Tracking

**Problem**: Losing context between development sessions, forgetting key decisions and learnings.

**Solution**: Use insights to capture session context and decisions.

```bash
# Start of day - capture context
insights add "daily" "$(date +%Y-%m-%d)-start" "Starting work on user authentication" "Current state: API routes implemented, need to add JWT validation. Blockers: unclear token expiration strategy. Goals: finish auth middleware, write tests"

# During development - capture key decisions
insights add "architecture" "jwt-token-expiration" "Decided on JWT token strategy" "Using 1-hour access tokens with 7-day refresh tokens. Access tokens in memory, refresh tokens in httpOnly cookies. Reasoning: balance between security and UX"

# End of day - capture progress and next steps
insights add "daily" "$(date +%Y-%m-%d)-end" "Completed auth middleware implementation" "Completed: JWT validation middleware, error handling, basic tests. Next: implement refresh token rotation, add rate limiting. Key learning: middleware order matters for error handling"
```

**Search and retrieve context:**
```bash
# Next day - get back up to speed
insights search "authentication" --topic daily
insights search "JWT strategy" --semantic
```

### Code Review and Quality Management

**Problem**: Inconsistent code quality checks, forgetting review patterns.

**Solution**: Integrated quality workflow with automated tasks and insight capture.

```bash
# Pre-commit quality check
blizz do checks   # runs format, lint, test, violet

# Capture review patterns
insights add "code-review" "error-handling-pattern" "Consistent error handling pattern" "Established pattern: all API routes use Result<T, ApiError>, centralized error handling middleware converts to HTTP responses. Prevents inconsistent error formats across endpoints"

# Document quality violations and solutions
insights add "code-quality" "violet-complexity-fix" "Reduced function complexity in user service" "violet flagged UserService::create_user as too complex. Split into: validate_user_data(), hash_password(), save_to_database(). Each function now has single responsibility and better testability"
```

### Learning and Knowledge Capture

**Problem**: Forgetting solutions to problems, repeating research.

**Solution**: Systematic knowledge capture with searchable insights.

```bash
# Capture debugging solutions
insights add "debugging" "docker-port-binding-issue" "Fixed Docker port binding in development" "Error: 'port already in use' when running docker-compose up. Solution: docker-compose down --volumes to clean up orphaned containers. Also added 'restart: unless-stopped' to avoid manual cleanup"

# Document integration solutions
insights add "integrations" "stripe-webhook-verification" "Implementing Stripe webhook signature verification" "Must verify webhook signatures for security. Steps: 1) Get endpoint secret from Stripe dashboard, 2) store in secrets manager, 3) use stripe.Webhook.constructEvent() with raw body. Key gotcha: must use raw request body, not parsed JSON"

# Capture performance optimizations
insights add "performance" "database-query-optimization" "Optimized user dashboard query" "Dashboard was loading 3+ seconds. Problem: N+1 queries for user projects. Solution: single query with LEFT JOIN, reduced from 15 queries to 1. Load time now <300ms. Pattern: always check query count in development"
```

## Team Workflows

### Onboarding New Team Members

**Setup team knowledge sharing** (when team features are available):
```bash
# Share common team insights
insights add "team-onboarding" "development-setup" "Standard development environment setup" "Required tools: Docker, Node 18+, PostgreSQL 14. Setup steps: 1) clone repo, 2) cp .env.example .env, 3) docker-compose up -d, 4) npm run setup. Common issues: port conflicts (stop local postgres), file permissions (chmod +x scripts/*)"

insights add "team-patterns" "code-review-checklist" "Code review standards" "Always check: 1) Tests added/updated, 2) Error handling implemented, 3) violet --quiet passes, 4) No hardcoded secrets, 5) Documentation updated if needed. Focus areas: security (auth/validation), performance (query efficiency), maintainability (violet score)"
```

**New team member workflow:**
```bash
# Day 1 - capture setup experience
insights add "team-onboarding" "my-setup-experience" "First day setup notes" "Setup time: 45 minutes. Issues encountered: Docker port conflict (fixed with docker system prune), Node version wrong (used nvm). Suggestions: add Node version to README, mention Docker cleanup step"

# Week 1 - document learning
insights search "development setup"  # find team patterns
insights search "code review" --topic team-patterns
```

### Architecture Decision Records

**Problem**: Losing context around architectural decisions, repeating discussions.

**Solution**: Use insights as lightweight ADRs (Architecture Decision Records).

```bash
insights add "architecture" "database-choice-postgresql" "Chose PostgreSQL over MongoDB" "Decision: Use PostgreSQL for user data storage. Context: Need ACID transactions for billing, team familiar with SQL, JSON columns provide flexibility where needed. Alternatives considered: MongoDB (less transaction support), MySQL (weaker JSON support). Decision date: 2024-01-15"

insights add "architecture" "api-versioning-strategy" "API versioning via header" "Decision: Use Accept header for API versioning (Accept: application/vnd.api.v1+json). Context: Avoids URL pollution, follows REST principles. Implementation: middleware parses header, defaults to latest version. Migration strategy: maintain 2 versions max, 6-month deprecation cycle"
```

## Enterprise Workflows

### Compliance and Audit Trail

**Problem**: Need audit trail for security compliance, tracking access patterns.

**Solution**: Systematic documentation of security-relevant decisions and access patterns.

```bash
# Document security decisions
insights add "security" "authentication-audit-$(date +%Y-%m-%d)" "Security review findings" "Reviewed authentication system. Findings: JWT tokens properly signed, refresh rotation implemented, rate limiting on auth endpoints. Action items: add login attempt logging, implement account lockout policy. Compliance status: SOC2 requirements met"

# Track access pattern changes
insights add "security" "database-access-controls" "Updated database access permissions" "Changed: removed direct DB access for intern accounts, all access now via API. Reason: prepare for SOC2 audit. Affected users: [list]. Migration: updated connection strings, revoked DB credentials. Verified: all applications still functional"
```

### Performance Monitoring and Optimization

**Problem**: Performance issues are hard to track over time, solutions get forgotten.

**Solution**: Systematic performance insight capture with metrics.

```bash
# Document performance baselines
insights add "performance" "api-baseline-$(date +%Y-%m-%d)" "API performance baseline established" "Measured baseline: /api/users avg 120ms, /api/projects avg 350ms, /api/dashboard avg 800ms. Method: 1000 requests via Artillery. Next review: 2 weeks. Targets: <100ms, <300ms, <500ms respectively"

# Track optimization results
insights add "performance" "redis-caching-implementation" "Implemented Redis caching for user data" "Change: Added Redis cache for user profile lookups. Results: /api/users avg 45ms (was 120ms), cache hit rate 85%. Implementation: 5-minute TTL, cache invalidation on user updates. Memory usage: +50MB Redis. Cost/benefit: positive"
```

## Advanced Power User Workflows

### Cross-Project Knowledge Management

**Problem**: Working on multiple projects, losing context when switching.

**Solution**: Project-specific insights with cross-project search.

```bash
# Project-specific insights
cd /path/to/project-a
insights add "project-a" "microservice-communication" "Service communication patterns" "Project A uses event-driven architecture: events via AWS SQS, async processing with Lambda. Key patterns: idempotent handlers, dead letter queues, exponential backoff"

cd /path/to/project-b  
insights add "project-b" "microservice-communication" "Service communication patterns" "Project B uses synchronous HTTP with circuit breakers: Hystrix for resilience, service discovery via Consul. Key patterns: timeout configurations, fallback responses, health checks"

# Cross-project search
insights search "microservice communication" --semantic
# Finds patterns across both projects, helps identify best practices
```

### Automation and Continuous Learning

**Problem**: Repeating manual processes, not systematically improving workflows.

**Solution**: Automate insight capture and workflow optimization.

```bash
# Automated insight capture in CI/CD
# Add to .github/workflows/insights.yml:
# - name: Capture deployment insights
#   run: |
#     insights add "deployments" "deploy-$(date +%Y-%m-%d)-${{ github.sha }}" "Deployment completed" "Version: ${{ github.sha }}, Duration: ${DEPLOY_TIME}s, Issues: ${DEPLOY_ISSUES}, Performance: ${DEPLOY_METRICS}"

# Weekly workflow optimization
insights search "workflow" --semantic | grep -i "slow\|problem\|issue"
# Review findings, optimize based on patterns
```

### Integration with Development Tools

**VS Code Integration** (via tasks.json):
```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Capture Debug Session",
            "type": "shell",
            "command": "insights",
            "args": ["add", "debugging", "session-${input:timestamp}", "${input:title}", "${input:details}"],
            "group": "build"
        }
    ],
    "inputs": [
        {
            "id": "timestamp",
            "description": "Session timestamp",
            "default": "$(date +%Y-%m-%d-%H%M)"
        },
        {
            "id": "title", 
            "description": "Debug session title",
            "type": "promptString"
        },
        {
            "id": "details",
            "description": "Session details", 
            "type": "promptString"
        }
    ]
}
```

## Measurement and ROI

### Tracking Value Delivery

**Time Savings Measurement:**
```bash
# Weekly time savings assessment
insights add "metrics" "time-savings-$(date +%Y-%m-%d)" "Weekly time savings from Blizz" "Estimated time saved: X hours. Sources: faster context switching (insights search), avoided rework (captured decisions), faster code review (violet + captured patterns). ROI: time saved * hourly rate = $X value"
```

**Knowledge Base Growth:**
```bash
# Monthly knowledge base metrics
insights topics | wc -l  # Topic count
insights list | wc -l   # Total insights
insights search "solution" --exact | wc -l  # Solution count
```

These examples show how Blizz adapts to different scales of usage, from individual productivity to enterprise compliance requirements. The key is starting simple and building complexity as needed.

---

*Want to share your workflow? [Add it to our discussions](https://github.com/kernelle-soft/blizz/discussions) and help other users learn from your experience.*
