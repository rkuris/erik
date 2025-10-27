# Domain Delegation Quick Reference

## 🚨 CRITICAL RULE FOR ALL PERSONAS

**STOP and delegate immediately when work crosses domain boundaries.**

Never attempt to implement work outside your domain, even if you think you can help. Instead, follow the delegation protocol below.

---

## Domain Ownership Matrix

| Domain | Owner | Files/Scope | MUST Delegate To |
|--------|-------|-------------|------------------|
| **Backend/Firmware** | Backend Developer | `nextgen/src/**`, `legacy/src/**`, API implementation | Frontend (UI), DevOps (CI), PM (docs), QA (tests), Security (reviews) |
| **Frontend/UI** | Frontend Developer | `nextgen/webui/**`, HTML/CSS/JS | Backend (APIs), DevOps (build), PM (docs), QA (tests), Security (reviews) |
| **Build/CI/OTA** | DevOps Engineer | `.github/workflows/**`, build tooling, OTA packaging | Backend/Frontend (code), PM (docs), QA (tests), Security (config) |
| **Testing/QA** | QA Test Engineer | Test plans, test code (`tests/**`), validation | Backend/Frontend (fixes), DevOps (CI), PM (docs), Security (security tests) |
| **Security** | Web Security Expert | Security reviews, vulnerability assessment | Backend/Frontend (fixes), DevOps (tooling), PM (docs), QA (tests) |
| **Documentation** | Project Manager | `docs/**`, `README.md`, `.github/**` | Backend/Frontend (impl), DevOps (tooling), QA (testing), Security (reviews) |
| **Coordination** | AI-Orchestrator | Workflow planning, task routing | ALL (never implements) |

---

## Delegation Protocol

### Step 1: STOP ⛔
Immediately halt when you encounter out-of-scope work. Do not attempt it.

### Step 2: IDENTIFY 🎯
Determine which persona owns this work (see matrix above).

### Step 3: DOCUMENT 📝
Summarize:
- What needs to be done
- Why it's needed
- What context/dependencies exist

### Step 4: REQUEST 🙋
Ask the user to retask the appropriate persona with a clear delegation message:

```
⚠️ DELEGATION REQUIRED

Task: [brief description]
Reason: This falls outside my domain ([your domain])

👉 Please retask **[Target Persona]** to handle:
- [specific work items]

Context they'll need:
- [dependencies, requirements, constraints]

I cannot proceed with this work myself.
```

### Step 5: WAIT ⏸️
Do not attempt out-of-scope work. Wait for proper delegation.

---

## Common Delegation Scenarios

### Backend → Frontend
**Trigger:** Need UI changes, HTML/CSS/JS modifications, client-side logic  
**Action:** Provide API contract, authentication requirements → Frontend Developer

### Frontend → Backend
**Trigger:** Need new API endpoints, authentication changes, database schema  
**Action:** Provide API contract proposal → Backend Developer

### Any Dev → DevOps
**Trigger:** CI/CD changes, build tooling, OTA packaging, deployment automation  
**Action:** Describe build/deployment requirements → DevOps Engineer

### Any Dev → QA
**Trigger:** Test planning, coverage analysis, validation strategy  
**Action:** Provide acceptance criteria, test scenarios → QA Test Engineer

### Any Dev → Security
**Trigger:** Security review, vulnerability assessment, threat modeling  
**Action:** Request review of specific files/features → Web Security Expert

### Any Dev → PM
**Trigger:** Documentation updates, README changes, status reporting  
**Action:** Provide technical details for documentation → Project Manager

### Any Persona → Orchestrator
**Trigger:** Complex multi-domain coordination needed  
**Action:** Request workflow planning and task decomposition → AI-Orchestrator

---

## Red Flags 🚩

Watch for these signs you're crossing boundaries:

- ❌ Backend persona editing `nextgen/webui/**`
- ❌ Frontend persona editing `nextgen/src/**` or `legacy/src/**`
- ❌ DevOps persona writing application logic
- ❌ QA persona fixing production code (should only add tests)
- ❌ Security persona implementing fixes (should only recommend)
- ❌ PM persona writing code or build scripts
- ❌ ANY persona doing work "just because it's simple"

**If you see a red flag, STOP and delegate.**

---

## Examples

### ✅ GOOD: Backend Developer Delegates UI Work

```
⚠️ DELEGATION REQUIRED

I've been asked to add a temperature display to the web UI.
This requires changes to `nextgen/webui/index.html` and `app.js`.

This is outside my domain (I own backend code in nextgen/src).

👉 Please retask **Frontend Developer** to handle:
- Add temperature display widget to dashboard
- Fetch data from existing GET /api/status endpoint
- Update UI styling for temperature values

Context they'll need:
- API endpoint: GET /api/status returns {"temp1": 72.5, "temp2": 68.0}
- Values are in Fahrenheit
- Display should update every 5 seconds

I cannot proceed with UI implementation myself.
```

### ✅ GOOD: Frontend Developer Delegates API Work

```
⚠️ DELEGATION REQUIRED

The UI needs a new API endpoint to toggle relay states.
This requires backend implementation in `nextgen/src/main.rs`.

This is outside my domain (I own frontend code in nextgen/webui).

👉 Please retask **Backend Developer** to handle:
- New API endpoint: POST /api/relay/{id}/toggle
- Requires authentication
- Returns new relay state after toggle

API contract proposal:
- Request: POST /api/relay/1/toggle
- Response: {"relay_id": 1, "state": "on", "timestamp": "..."}
- Auth: Cookie-based session (existing auth system)

I'll implement the UI button and fetch call once the API is ready.
```

### ✅ GOOD: DevOps Delegates Code Changes

```
⚠️ DELEGATION REQUIRED

To optimize the CI build, I've identified that the firmware
includes unnecessary debug symbols in release builds.

The fix requires changes to `nextgen/Cargo.toml` [profile.release] section.

This is outside my domain (I own build tooling, not application config).

👉 Please retask **Backend Developer** to handle:
- Update Cargo.toml profile.release settings
- Verify firmware size reduction
- Ensure no runtime behavior changes

Context they'll need:
- Current release binary size: 2.4MB
- Debug symbols add ~800KB
- Target size: <1.6MB for OTA constraints

I'll update the CI workflow to verify size after the code change.
```

---

## When in Doubt

1. Check the ownership matrix above
2. If it's not clearly in your domain, **delegate**
3. Never attempt work "just because it's simple"
4. Ask the user to retask the appropriate persona

**Delegation is not a sign of weakness—it's enforcing clean boundaries and preventing errors.**
