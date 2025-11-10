# Domain Delegation Quick Reference

## 🚨 CRITICAL RULE FOR ALL PERSONAS

**STOP and delegate immediately when work crosses domain boundaries.**

Never attempt to implement work outside your domain, even if you think you can help. Instead, follow the delegation protocol below.

---

## Domain Ownership Matrix

| Domain | Owner | Files/Scope | MUST Delegate To |
|--------|-------|-------------|------------------|
| **Backend/Firmware** | Backend Developer | `nextgen/src/**`, `legacy/src/**`, API implementation | Frontend (UI), Build Systems (CI/CD), Platform (partition config), PM (docs), QA (tests), Security (reviews) |
| **Frontend/UI** | Frontend Developer | `nextgen/webui/**`, HTML/CSS/JS | Backend (APIs), Build Systems (build), Platform (hardware constraints), PM (docs), QA (tests), Security (reviews) |
| **Build/CI/OTA** | Build Systems Engineer | `.github/workflows/**`, compiler flags, size optimizations, OTA packaging | Backend/Frontend (code), Platform (partition/memory), PM (docs), QA (tests), Security (config) |
| **Platform/Hardware** | Platform Engineer | Partition layout, memory allocation, flash/RAM constraints, `partitions.csv`, `sdkconfig.defaults` (memory-related) | Backend/Frontend (code), Build Systems (compiler flags), PM (docs), QA (tests), Security (secure boot/flash encryption) |
| **Testing/QA** | QA Test Engineer | Test plans, test code (`tests/**`), validation | Backend/Frontend (fixes), Build Systems/Platform (CI/tooling), PM (docs), Security (security tests) |
| **Security** | Web Security Expert | Security reviews, vulnerability assessment | Backend/Frontend (fixes), Build Systems (security tooling), Platform (secure boot/flash encryption), PM (docs), QA (tests) |
| **Documentation** | Project Manager | `docs/**`, `README.md`, `.github/**` | Backend/Frontend (impl), Build Systems (tooling), Platform (hardware config), QA (testing), Security (reviews) |

**⚠️ MIGRATION NOTE (2025-11-09):**  
The former "DevOps Engineer" role has been split into two specialized personas:
- **All new build optimization tasks** → Build Systems Engineer
- **All partition/layout/flash capacity tasks** → Platform Engineer

Both personas must cross-reference when changes affect overlapping concerns (e.g., binary size reduction requiring partition resizing).

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

### Any Dev → Build Systems
**Trigger:** CI/CD changes, compiler optimizations, size reduction, build tooling, OTA packaging  
**Action:** Describe build/optimization requirements → Build Systems Engineer

### Any Dev → Platform
**Trigger:** Partition layout changes, memory allocation, flash/RAM constraints, hardware config  
**Action:** Describe memory/partition requirements → Platform Engineer

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

## FAQ

### Why split the DevOps role into Build Systems and Platform Engineers?

**Specialization reduces thrash and improves accountability:**

1. **Different expertise domains:** Compiler optimization (LLVM flags, LTO, strip settings) requires different skills than partition layout (flash geometry, OTA slot sizing, bootloader constraints).

2. **Clear ownership boundaries:** When a firmware image is 200 KB too large, Build Systems Engineer optimizes compiler output while Platform Engineer may need to resize partitions—splitting these prevents confusion about who owns what.

3. **Cross-reference coordination:** Size/partition issues often require both personas to collaborate. Explicit cross-reference protocol ensures they coordinate rather than work in conflict.

4. **Historical context:** Early in the project, size optimization tasks and partition layout changes were sometimes duplicated or mis-assigned. The split eliminates this ambiguity.

**Result:** Faster iteration, clearer escalation paths, and reduced risk of conflicting changes to build config vs. partition layout.

---

## Red Flags 🚩

Watch for these signs you're crossing boundaries:

- ❌ Backend persona editing `nextgen/webui/**`
- ❌ Frontend persona editing `nextgen/src/**` or `legacy/src/**`
- ❌ Build Systems persona writing application logic or editing partition layout
- ❌ Platform persona editing compiler flags or application code
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

### ✅ GOOD: Build Systems Engineer Delegates Code Changes

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

### ✅ GOOD: Platform Engineer Coordinates Size Reduction

```
⚠️ DELEGATION REQUIRED

Firmware image exceeds partition budget by 150 KB even after partition resize.

This requires compiler optimization changes in `sdkconfig.defaults` and Cargo profile.

This is outside my domain (I own partition layout, not compiler config).

👉 Please retask **Build Systems Engineer** to handle:
- Enable aggressive size optimizations in sdkconfig.defaults
- Review Cargo.toml release profile for LTO/strip settings
- Target reduction: 150+ KB from current 1.91 MB

Context they'll need:
- Partition budget: 0x1F0000 bytes (2,064,384 bytes / 1.97 MB)
- Current image: 2,003,685 bytes (~1.91 MB)
- Headroom target: <1.8 MB to allow for growth

I've confirmed partition layout is optimal and cannot be expanded further.
```

---

## When in Doubt

1. Check the ownership matrix above
2. If it's not clearly in your domain, **delegate**
3. Never attempt work "just because it's simple"
4. Ask the user to retask the appropriate persona

**Delegation is not a sign of weakness—it's enforcing clean boundaries and preventing errors.**
