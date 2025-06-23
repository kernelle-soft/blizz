# Jerrod GitHub Reaction System

Jerrod uses GitHub reactions as a semantic communication system during merge request reviews. Each reaction has a specific meaning that helps organize review workflow and provides clear signals about comment status.

## Reaction Meanings

| Emoji | Semantic Meaning | Usage |
|-------|------------------|-------|
| 👍 | **Acknowledged/Agreed** | Comment has been read and acknowledged. Used for non-actionable comments or general agreement. |
| 👎 | **Disagreement** | Disagreement with the comment or approach. Requires discussion or resolution. |
| 😄 | **Humor/Light** | Comment is humorous or light-hearted. Usually non-actionable. |
| 🎉 | **Celebration** | Celebrating completion, good work, or milestone. Positive acknowledgment. |
| 😕 | **Confusion/Concern** | Comment raises confusion or concern. May need clarification or discussion. |
| ❤️ | **Appreciation** | Strong appreciation or love for the comment/work. Positive feedback. |
| 🚀 | **Ready/Ship It** | Work is ready to proceed, ship, or merge. Strong approval signal. |
| 👀 | **Noted/Watching** | Comment has been seen and noted. Used for informational comments that don't require action but should be tracked. |

## Specialized Workflow Reactions

### Review Response Workflow

For top-level MR comments that reference specific review threads, jerrod uses a look-ahead/look-behind system:

#### Pattern: `[Reaction]: [Link]`

Comments with reactions followed by links are pointing to the explicit comment they're replying to:

```markdown
👀: https://github.com/owner/repo/pull/123#issuecomment-456789

This comment is noted but not actionable in this MR.
```

```markdown
🚀: https://github.com/owner/repo/pull/123#issuecomment-456789  

Great implementation! Ready to ship this feature.
```

#### Response Templates

**Deferred Work**:
```markdown
🗒️: https://github.com/owner/repo/pull/123#issuecomment-456789

Got it! This optimization is unrelated to the current feature. 
I've created a separate issue for it: https://github.com/owner/repo/issues/456
```

**Follow-up Questions**:
```markdown
❓: https://github.com/owner/repo/pull/123#issuecomment-456789

Quick clarification: Should this validation happen on the client side 
or server side? Current implementation does client-side validation.
```

**Acknowledged Non-Actionable**:
```markdown
👀: https://github.com/owner/repo/pull/123#issuecomment-456789

Noted. This is architectural context for future development.
```

**Fully Addressed**:
```markdown
✅: https://github.com/owner/repo/pull/123#issuecomment-456789

Fixed in commit abc123. Updated the validation logic as requested.
```

## Command Usage

### Acknowledge Command

The `jerrod acknowledge` command adds appropriate reactions to review comments:

```bash
# Add thumbs up (default acknowledgment)
jerrod acknowledge

# Specific reaction types
jerrod acknowledge --eyes      # 👀 (noted/watching)
jerrod acknowledge --rocket    # 🚀 (ready/ship it)
jerrod acknowledge --heart     # ❤️ (appreciation)
jerrod acknowledge --confused  # 😕 (needs clarification)
```

### Workflow Integration

1. **Review Comments**: Use reactions to indicate comment status
2. **Thread Resolution**: Reactions help filter resolved vs. unresolved threads
3. **Communication**: Clear semantic meaning reduces back-and-forth
4. **Automation**: Tools can filter and process comments based on reactions

## Best Practices

### For Reviewers
- Use 👍 for general acknowledgment of non-actionable comments
- Use 👀 for informational comments that should be tracked
- Use 🚀 when work is ready to proceed
- Use 😕 when clarification is needed

### For Authors
- Respond to reactions appropriately in follow-up commits
- Use reaction-link patterns for top-level response comments
- Mark completion with ✅ reactions when addressing feedback
- Create issues for deferred work and link them with 🗒️ reactions

### For Teams
- Establish consistent reaction meanings across projects
- Use reactions to reduce notification noise
- Filter comments by reaction status during review cycles
- Train team members on semantic meanings

## Technical Implementation

Jerrod maps these emoji reactions to GitHub's reaction API:

- 👍 → `+1`
- 👎 → `-1` 
- 😄 → `laugh`
- 🎉 → `hooray`
- 😕 → `confused`
- ❤️ → `heart`
- 🚀 → `rocket`
- 👀 → `eyes`

The reaction system works across GitHub's web interface, mobile apps, and API integrations, ensuring consistency regardless of access method. 