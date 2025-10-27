#!/bin/bash
# Bulk create GitHub issues from task documentation
# Usage: ./create-project-issues.sh

set -e

REPO="rkuris/erik"
DOCS_BASE="/workspaces/pool-controller/docs"

echo "Creating issues for rkuris/erik..."

# Security tasks (6 incomplete)
echo -e "\n=== Creating Security Task Issues ==="

gh issue create --repo "$REPO" \
  --title "Session Hardening & Timeout" \
  --body-file "$DOCS_BASE/security/Incomplete/security-task-04-session-hardening.md" \
  --label "security,persona:backend,persona:security" \
  --assignee "" || echo "Failed to create issue 1"

gh issue create --repo "$REPO" \
  --title "Firmware Upload Validation" \
  --body-file "$DOCS_BASE/security/Incomplete/security-task-05-firmware-upload.md" \
  --label "security,persona:backend,persona:security" \
  --assignee "" || echo "Failed to create issue 2"

gh issue create --repo "$REPO" \
  --title "CSRF Mitigation" \
  --body-file "$DOCS_BASE/security/Incomplete/security-task-06-csrf-mitigation.md" \
  --label "security,persona:backend,persona:frontend,persona:security" \
  --assignee "" || echo "Failed to create issue 3"

gh issue create --repo "$REPO" \
  --title "Operational Hardening" \
  --body-file "$DOCS_BASE/security/Incomplete/security-task-07-operational-hardening.md" \
  --label "security,persona:backend,persona:security" \
  --assignee "" || echo "Failed to create issue 4"

gh issue create --repo "$REPO" \
  --title "Login Rate Limiting" \
  --body-file "$DOCS_BASE/security/Incomplete/security-task-08-login-rate-limiting.md" \
  --label "security,persona:backend,persona:security" \
  --assignee "" || echo "Failed to create issue 5"

gh issue create --repo "$REPO" \
  --title "Transport Layer Hardening" \
  --body-file "$DOCS_BASE/security/Incomplete/security-task-09-transport-hardening.md" \
  --label "security,persona:backend,persona:devops,persona:security" \
  --assignee "" || echo "Failed to create issue 6"

# Wi-Fi & OTA tasks (10 tasks)
echo -e "\n=== Creating Wi-Fi & OTA Task Issues ==="

gh issue create --repo "$REPO" \
  --title "Wi-Fi State Machine Implementation" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-01-wifi-state-machine.md" \
  --label "wifi-provisioning,persona:backend" \
  --assignee "" || echo "Failed to create issue 7"

gh issue create --repo "$REPO" \
  --title "Wi-Fi Status Endpoint" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-02-status-endpoint.md" \
  --label "wifi-provisioning,persona:backend" \
  --assignee "" || echo "Failed to create issue 8"

gh issue create --repo "$REPO" \
  --title "Wi-Fi Configuration API" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-03-wifi-api.md" \
  --label "wifi-provisioning,persona:backend" \
  --assignee "" || echo "Failed to create issue 9"

gh issue create --repo "$REPO" \
  --title "Wi-Fi Setup UI" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-04-wifi-ui.md" \
  --label "wifi-provisioning,persona:frontend" \
  --assignee "" || echo "Failed to create issue 10"

gh issue create --repo "$REPO" \
  --title "Wi-Fi Integration Tests" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-05-wifi-tests.md" \
  --label "wifi-provisioning,testing,persona:qa" \
  --assignee "" || echo "Failed to create issue 11"

gh issue create --repo "$REPO" \
  --title "OTA Partition Scheme" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-06-ota-partitions.md" \
  --label "ota-updates,persona:backend,persona:devops" \
  --assignee "" || echo "Failed to create issue 12"

gh issue create --repo "$REPO" \
  --title "Secure Firmware Upload" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-07-secure-firmware-upload.md" \
  --label "ota-updates,security,persona:backend,persona:security" \
  --assignee "" || echo "Failed to create issue 13"

gh issue create --repo "$REPO" \
  --title "OTA Rollback Mechanism" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-08-ota-rollback.md" \
  --label "ota-updates,persona:backend" \
  --assignee "" || echo "Failed to create issue 14"

gh issue create --repo "$REPO" \
  --title "OTA Update UI" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-09-ota-ui.md" \
  --label "ota-updates,persona:frontend" \
  --assignee "" || echo "Failed to create issue 15"

gh issue create --repo "$REPO" \
  --title "OTA Smoke Test Suite" \
  --body-file "$DOCS_BASE/wifi-ota/Incomplete/task-10-ota-smoketest.md" \
  --label "ota-updates,testing,persona:qa" \
  --assignee "" || echo "Failed to create issue 16"

echo -e "\n✅ All issues created successfully!"
echo "Next steps:"
echo "  1. Create GitHub Project board"
echo "  2. Add issues to the project"
echo "  3. Configure custom fields and views"
