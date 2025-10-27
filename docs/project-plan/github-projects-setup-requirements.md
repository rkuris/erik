# GitHub Projects Board Setup Requirements

**Owner:** DevOps Engineer  
**Status:** Ready for implementation  
**Date:** 2025-10-26

## Objective

Create a GitHub Projects board for the `rkuris/erik` repository to visualize and track all tasks across security remediation, Wi-Fi/OTA development, and phased implementation work.

---

## Board Structure

### Recommended Layout: **Table View + Kanban View**

**Table View** for detailed tracking:
- Shows all fields (assignee, labels, phase, persona, priority)
- Filterable by persona, phase, or task type

**Kanban View** for workflow visualization:
- Columns: `Backlog` → `Todo` → `In Progress` → `In Review` → `Done`

---

## Issue Sources

Create GitHub Issues from these existing task documents:

### 1. Security Tasks (`docs/security/`)
- **Completed (3):** Already in `Complete/` folder
- **Active (6):** Currently in `Incomplete/` folder
  - security-task-04-session-hardening.md
  - security-task-05-firmware-upload.md
  - security-task-06-csrf-mitigation.md
  - security-task-07-operational-hardening.md
  - security-task-08-login-rate-limiting.md
  - security-task-09-transport-hardening.md

### 2. Wi-Fi & OTA Tasks (`docs/wifi-ota/`)
- **All 10 tasks** currently in `Incomplete/`
  - task-01-wifi-state-machine.md
  - task-02-status-endpoint.md
  - task-03-wifi-api.md
  - task-04-wifi-ui.md
  - task-05-wifi-tests.md
  - task-06-ota-partitions.md
  - task-07-secure-firmware-upload.md
  - task-08-ota-rollback.md
  - task-09-ota-ui.md
  - task-10-ota-smoketest.md

### 3. Phase-Based Tasks (`docs/project-plan/phase-**/`)
- 7 phases × 5 personas = ~35 task documents
- These provide acceptance criteria and persona-specific checklists

---

## Custom Fields Needed

| Field Name | Type | Values | Purpose |
|------------|------|--------|---------|
| **Persona** | Single Select | Backend, Frontend, DevOps, QA, Security, PM | Track ownership |
| **Phase** | Single Select | Phase 1-7, Ongoing | Map to project roadmap |
| **Task Type** | Single Select | Security, Wi-Fi, OTA, Infrastructure, Documentation | Categorize work |

Note: Add `Testing` to the Task Type single-select to track QA/test infrastructure and automation work.
| **Priority** | Single Select | Critical, High, Medium, Low | Triage order |
| **Blocked By** | Text | Issue #, dependency description | Track blockers |

---

## Labels to Create

**By Persona:**
- `persona:backend`
- `persona:frontend`
- `persona:devops`
- `persona:qa`
- `persona:security`
- `persona:pm`

**By Work Type:**
- `security`
- `wifi-provisioning`
- `ota-updates`
- `infrastructure`
- `documentation`
- `testing`

**By Status:**
- `blocked`
- `needs-review`
- `ready-to-merge`

---

## Automation Rules (Optional)

If GitHub Actions are preferred:
1. Auto-move issues to "In Progress" when branch is created
2. Auto-move to "In Review" when PR is opened
3. Auto-move to "Done" when PR is merged
4. Auto-label based on file paths (e.g., `nextgen/src/**` → `persona:backend`)

---

## Issue Template Integration

Use existing templates in `.github/ISSUE_TEMPLATE/` or create new ones for:
- Security remediation tasks
- Feature implementation tasks
- Bug reports
- Documentation updates

---

## Migration Strategy

### Phase 1: Initial Setup
1. Create GitHub Project board
2. Set up custom fields and labels
3. Configure kanban columns

### Phase 2: Bulk Import
1. Create issues for all security tasks (link to task docs)
2. Create issues for all Wi-Fi/OTA tasks
3. Optionally create meta-issues for each phase

### Phase 3: Team Onboarding
1. Document workflow (how to move cards, update fields)
2. Share board link with stakeholders
3. Establish weekly review cadence

---

## Success Criteria

- ✅ All active tasks visible on kanban board
- ✅ Each issue links back to its task document
- ✅ Filters work for persona-specific views
- ✅ Board URL is documented in README.md
- ✅ Team can self-service status updates

---

## References

- GitHub Projects docs: https://docs.github.com/en/issues/planning-and-tracking-with-projects
- Security task index: `docs/security/security-remediation-plan.md`
- Wi-Fi/OTA task index: `docs/wifi-ota/README.md`
- Phase planning: `docs/project-plan/phase-*/`

---

## Next Actions

**For DevOps Engineer:**
1. Review this requirements doc
2. Create the GitHub Project board (manual or via GH CLI)
3. Bulk-create issues from task documents (can use script or manual entry)
4. Configure automation rules
5. Update `README.md` with board link

**Dependencies:**
- Repository admin access to `rkuris/erik`
- Decision on whether to use GitHub Actions for automation

**Estimated effort:** 2-4 hours for initial setup + bulk import
