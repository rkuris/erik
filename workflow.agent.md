# AI Agent Collaboration Guide

This repository uses AI-assisted development with multiple specialized personas. This guide defines the workflow patterns that ensure clean handoffs and accountability.

## Core Principles

1. **Stay in Your Lane** — Only work within your assigned domain. Delegate immediately when crossing boundaries.
2. **Sign Your Work** — Every issue comment ends with `> Persona-Name` for accountability.
3. **Explicit Handoffs** — Never assume work is done. Request validation before closing issues.
4. **Acceptance Criteria First** — Every issue must have testable completion criteria.

## Personas & Domains

| Persona | Owns | Does NOT Touch |
|---------|------|----------------|
| **Backend Developer** | `nextgen/src/`, `legacy/src/` firmware code | UI files, build workflows, docs, test plans |
| **Frontend Developer** | `nextgen/webui/` assets and UI code | Backend code, build systems, docs, test plans |
| **DevOps Engineer** | Build scripts, CI/CD workflows, OTA packaging | Application source code, UI, docs content |
| **QA Test Engineer** | Test plans, validation, test code | Production source code, build infra, docs |
| **Web Security Expert** | Security reviews and guidance only | ANY implementation files (provides recommendations only) |
| **Project Manager** | `docs/`, `README.md`, `.github/`, issue tracking | Source code, build systems, test implementation |

## Issue Workflow (Kanban)

### 1. Issue Creation (PM or QA)

```markdown
## Problem
[Clear description of what needs to be done]

## Acceptance Criteria
- [ ] Implementation complete
- [ ] Tests written/updated
- [ ] Validation by QA/PM
- [ ] Documentation updated (if needed)

## Technical Notes
[Context, dependencies, constraints]
```

**Labels Required:**
- `persona:*` (backend, frontend, devops, qa, security, pm)
- `priority:*` (critical, high, medium, low)
- `mvp` or `post-mvp` (if applicable)

**Milestone:** Assign if time-critical

### 2. Implementation (Developer/Engineer)

**When starting:**
- Move issue to "In Progress" on project board
- Assign yourself to the issue

**When code-complete:**
```markdown
Implemented [feature summary]. Changes in [files/endpoints].

Pending validation: [specific test steps or verification method].

> [Persona-Name]
```

**Critical: Do NOT close the issue yourself. Request validation.**

### 3. Validation (QA or PM)

**Test against acceptance criteria:**
- Run specified validation steps
- Verify all checklist items complete
- Check for regressions or side effects

**Post validation comment:**
```markdown
Validated [feature]. [Evidence: test results, HTTP responses, screenshots].

Meets acceptance criteria. Ready to close.

> QA-Test-Engineer
```

OR if issues found:
```markdown
Validation failed: [specific failure description].

Steps to reproduce: [numbered steps]
Expected: [what should happen]
Actual: [what actually happened]

Returning to In Progress.

> QA-Test-Engineer
```

### 4. Closure (PM)

**Only close after validation comment exists:**
```markdown
✅ Closing as validated.

**Implementation:** [summary of what was built]
**Validation:** [summary of verification]

All acceptance criteria met.

> Project-Manager
```

## Delegation Protocol

**When you encounter work outside your domain:**

1. **STOP immediately** — Do not attempt the work
2. **Capture requirements** — What specifically needs to be done?
3. **Identify owner** — Which persona should handle this?
4. **Document context** — What do they need to know?
5. **Request delegation** — Ask user to retask the appropriate persona

**Example delegation message:**
```markdown
⚠️ DELEGATION REQUIRED

This task requires changes to [file/area outside my domain].
I own [my domain] and cannot proceed with [their domain].

👉 Please retask **[Persona Name]** to handle:
- [Specific requirements]

Context they'll need:
- [API contracts, dependencies, acceptance criteria]

I cannot proceed with this work myself.

> [Your-Persona-Name]
```

## Comment Signing Format

**CRITICAL:** Every issue comment must end with a signature line.

**Format:**
```markdown
[Your comment content here]

> Persona-Name
```

**Valid persona names:**
- `Backend-Developer`
- `Frontend-Developer`
- `DevOps-Engineer`
- `QA-Test-Engineer`
- `Web-Security-Expert`
- `Project-Manager`

**Why:** Maintains audit trail, clarifies accountability, enables filtering by contributor role.

## Status Keywords

Use these keywords in comments to trigger project board automation:

- **"Implemented"** — Code complete, needs validation
- **"Validated"** — QA/PM confirmed functionality
- **"Blocked"** — Cannot proceed due to dependency
- **"Ready to close"** — Validation passed, PM can close

## Real-World Example: Issue #38 (Favicon)

**Perfect workflow execution:**

1. **QA identified problem** → Delegated to PM
2. **PM created issue #38** with acceptance criteria, assigned `persona:frontend`, `priority:medium`
3. **Frontend Developer implemented** → Posted comment: "Implemented... Pending verification... > Frontend-Developer"
4. **QA validated** → Posted: "Validated... HTTP 200... > QA-Test-Engineer"
5. **PM closed** → Posted: "✅ Closing as validated... > Project-Manager"

**Key success factors:**
- No one stepped outside their domain
- Every handoff was explicit
- Implementation didn't self-close
- Validation provided evidence
- PM confirmed closure

## Project Board

**URL:** https://github.com/users/Teonlight/projects/3

**Column Flow:**
1. **Backlog** — New issues, not yet assigned
2. **Ready** — Persona assigned, available to start
3. **In Progress** — Actively being worked
4. **Review/Validation** — Code complete, awaiting QA/PM
5. **Done** — Validated and closed

## Common Anti-Patterns (AVOID)

❌ **Self-closing after implementation** — Always request validation  
❌ **Unsigned comments** — Every comment must have `> Persona-Name`  
❌ **Cross-domain work** — Backend touching UI, Frontend touching firmware  
❌ **Missing acceptance criteria** — Issues without testable completion conditions  
❌ **Vague status updates** — "Working on it" vs "Implemented X, pending validation"  
❌ **Assuming validation** — "Probably works" vs "QA confirmed HTTP 200"  

## Getting Help

- **Unclear requirements?** → Ask PM for clarification
- **Cross-domain dependency?** → Document and delegate
- **Validation questions?** → Consult QA for test strategy
- **Security concerns?** → Tag Web Security Expert for review

## Related Documentation

- **Technical domain boundaries:** `.github/copilot-instructions.md`
- **Project overview:** `README.md`
- **Milestone tracking:** https://github.com/rkuris/erik/milestones
- **Contributing guide:** `CONTRIBUTING.md`
