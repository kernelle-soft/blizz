# Team Collaboration with Insights

Since insights are just markdown files, teams can collaborate on organizational knowledge using familiar git workflows.

## Team Insight Repository Pattern

### Setup Team Knowledge Sharing
```bash
# Create shared team insights repository
git init team-insights
cd team-insights

# Add initial team insights
insights add "architecture" "microservice-patterns" "Our microservice communication patterns" "We use async event-driven architecture with SQS for most inter-service communication. Sync HTTP only for user-facing APIs and real-time data needs."

insights add "deployment" "production-checklist" "Pre-deployment checklist" "1) Run full test suite, 2) Check monitoring dashboards, 3) Verify database migrations, 4) Update runbooks, 5) Notify team in #deployments"

# Commit and push
git add .
git commit -m "Initial team insights"
git push origin main
```

### Individual Developer Workflow
```bash
# Clone team insights to your local machine
cd ~/.blizz/insights/
git clone git@company.com:engineering/team-insights.git shared

# Link to your personal insights
# Now your AI agent has access to both personal and team knowledge

# Add personal insights that might benefit the team
insights add "debugging" "docker-memory-issues" "Fixing Docker memory issues on M1 Macs" "Docker Desktop on M1 Macs sometimes runs out of memory during builds. Solution: increase memory limit to 8GB in Docker Desktop settings, and add --platform=linux/amd64 to Dockerfile FROM statements"

# Share useful insights with team
cd ~/.blizz/insights/shared/
git add .
git commit -m "Add Docker M1 debugging solution"
git push origin main

# Pull team updates regularly  
git pull origin main
```

## Enterprise Collaboration Patterns

### Department-Level Knowledge Sharing
```bash
# Different insight repositories for different scopes
~/.blizz/insights/
├── personal/           # Your personal insights
├── team-backend/       # Backend team shared insights  
├── team-platform/      # Platform team shared insights
├── company-wide/       # Company-wide engineering insights
└── project-payments/   # Project-specific insights
```

### Compliance and Audit Trail
```bash
# Full git history shows who added what knowledge when
git log --oneline insights/security/

# Audit what knowledge AI agents have access to
find ~/.blizz/insights/ -name "*.md" | grep security
grep -r "password\|credential\|secret" ~/.blizz/insights/

# Remove sensitive insights that shouldn't be shared
git rm insights/internal/production-passwords.md
git commit -m "Remove sensitive credential information"
```

### Knowledge Review Process
```bash
# Use git branches for insight review
git checkout -b feature/new-deployment-process
insights add "deployment" "blue-green-deployment" "New blue-green deployment process" "..."
git add .
git commit -m "Add blue-green deployment insights"
git push origin feature/new-deployment-process

# Create pull request for team review
# Merge after approval - standard code review process for knowledge
```

## Advanced Team Workflows

### Automated Insight Collection
```yaml
# .github/workflows/collect-insights.yml
name: Collect Team Insights
on:
  workflow_dispatch:
  schedule:
    - cron: '0 18 * * 5'  # Weekly on Friday

jobs:
  collect-insights:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Collect deployment insights
        run: |
          # Automatically create insights from deployment data
          DEPLOY_COUNT=$(git log --since="1 week ago" --grep="deploy" --oneline | wc -l)
          insights add "metrics" "weekly-deployments-$(date +%Y-%m-%d)" "Weekly deployment metrics" "Deployments this week: $DEPLOY_COUNT. Average: X per day. Notable issues: [list]"
```

### Cross-Team Knowledge Discovery
```bash
# Search across all team insights
find ~/.blizz/insights/ -name "*.md" -exec grep -l "authentication" {} \;

# Pipeline insights across teams
grep -r "microservice" ~/.blizz/insights/ | \
  sed 's/.*insights\///' | \
  cut -d/ -f1 | \
  sort | uniq -c

# Find similar solutions across teams
insights search "API rate limiting" --topic all
```

### Migration and Backup
```bash
# Backup team insights (they're just files!)
tar -czf team-insights-backup-$(date +%Y-%m-%d).tar.gz ~/.blizz/insights/

# Migrate to new knowledge management system
# No vendor lock-in - just markdown files
rsync -av ~/.blizz/insights/ /new/knowledge/system/

# Convert to other formats if needed
find ~/.blizz/insights/ -name "*.md" -exec pandoc {} -o {}.html \;
```

## Best Practices for Team Collaboration

### Naming Conventions
```bash
# Use consistent topic/name patterns
insights add "deployment" "prod-hotfix-2024-01-15" "Production hotfix procedure" "..."
insights add "architecture" "database-migration-strategy" "Database migration approach" "..."
insights add "debugging" "memory-leak-investigation" "Memory leak debugging process" "..."
```

### Knowledge Curation
- **Weekly insight reviews**: Team reviews and updates outdated insights
- **Insight templates**: Standardized formats for common insight types  
- **Knowledge ownership**: Teams own insights for their domain areas
- **Deprecation process**: Mark outdated insights and remove obsolete information

### Privacy Boundaries
- **Personal insights**: Keep in personal folders, don't commit to shared repos
- **Sensitive information**: Use `.gitignore` for credentials, internal details
- **Access control**: Use separate repositories for different access levels
- **Audit compliance**: Regular reviews of shared insights for sensitive data

## Enterprise Integration Examples

### With Existing Documentation
```bash
# Convert existing runbooks to insights
pandoc runbooks/deployment.md -t markdown | \
  insights add "deployment" "legacy-runbook" "Deployment runbook" "$(cat -)"

# Keep insights synchronized with official docs
# Use git hooks to update insights when documentation changes
```

### With Monitoring and Alerting
```bash
# Automatically create insights from incident reports
curl -H "Authorization: Bearer $PAGER_DUTY_TOKEN" \
     "https://api.pagerduty.com/incidents" | \
  jq '.incidents[] | select(.status == "resolved")' | \
  xargs -I {} insights add "incidents" "incident-{id}" "{title}" "{summary}"
```

This markdown + git foundation makes Blizz incredibly enterprise-friendly - teams get AI-powered knowledge management that integrates with their existing workflows, with zero vendor lock-in risk.
