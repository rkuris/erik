This folder contains a minimal Playwright test scaffold for the static Web UI.

How to run locally

1. Install Node (recommended v18+). From the `nextgen/webui` directory run:

```bash
npm ci
npx playwright install
```

2. Run the tests (this will start a small static server automatically via the Playwright config):

```bash
npm test
```

Notes and CI

- The Playwright config starts `http-server` on port 3000 before running tests. The test assumes `index.html` and its assets are reachable at `/`.
- In CI, ensure Node is installed and consider running `npx playwright install --with-deps` on Linux runners to provision browsers.

What I added

- `package.json` - dev dependencies and convenient scripts
- `playwright.config.js` - launches a local static server and sets baseURL
- `tests/index.spec.js` - a smoke test that verifies the login form and basic UI elements render

Docker image (optional)

You can run the tests inside a Docker container so you don't need to install Node or browsers locally. I added a Dockerfile and a small helper script.

Build the image (from repository root):

```bash
docker build -t solar-heater-webui-tests -f nextgen/webui/Dockerfile .
```

Run the tests with the default command (runs Playwright tests):

```bash
docker run --rm -v "$(pwd)/nextgen/webui:/workspace" -w /workspace solar-heater-webui-tests
```

Run a single test file or pass Playwright args:

```bash
docker run --rm -v "$(pwd)/nextgen/webui:/workspace" -w /workspace solar-heater-webui-tests npx playwright test tests/index.spec.js
```

Or use the included helper script (runs in-container if you mount workspace):

```bash
chmod +x nextgen/webui/run-tests.sh
./nextgen/webui/run-tests.sh
```

Notes:
- The Docker image uses `mcr.microsoft.com/playwright:latest` which includes browsers and required system deps. If your CI environment disallows this image, you can switch to a `node` base and run `npx playwright install --with-deps` but that will require additional apt packages.
- Mounting the workspace into `/workspace` means tests run against the current working tree without needing to rebuild the image after code changes.

Docker Compose / Makefile (recommended for Docker Desktop)

If you'd like simpler local iteration on Docker Desktop, use the included `docker-compose.yml` and `Makefile` in the `nextgen/webui` directory. They are configured to:

- Build the Playwright-based image
- Mount the local `webui` folder into the container so source changes are picked up without rebuilding
- Increase shared memory for Chromium (`shm_size: 1gb`)
- Persist HTML reports to `nextgen/webui/playwright-report`

Common commands (run from repo root):

Build the image:

```bash
make -C nextgen/webui build
```

Run the full suite (compose-managed; report saved to `playwright-report`):

```bash
make -C nextgen/webui test
```

Bring up (attached) - useful to watch logs live:

```bash
make -C nextgen/webui up
```

Clean containers and local image (warning: removes image):

```bash
make -C nextgen/webui clean
```

Tips for Docker Desktop

- Ensure Docker Desktop has at least 2 CPU and 4GB RAM allocated. Increase shared memory if Chromium crashes frequently.
- If you get file permission issues when mounting the workspace, try the `--user $(id -u):$(id -g)` option on `docker run`, or use the Makefile which uses docker-compose (it should work in most setups).



## OTA Upload Workflow

The UI includes an OTA (Over-The-Air) firmware update feature available in the Admin view.

### Client-Side Validation

Before posting to the backend, the UI validates:

- **File size:** Maximum 2MB (configurable via `MAX_FIRMWARE_SIZE_MB` constant)
- **File magic:** Checks for ESP32 firmware magic bytes (`0xE9` at offset 0)
- **File type:** Must be a `.bin` file

Invalid uploads are rejected immediately with user-friendly error messages, preventing unnecessary network traffic.

### Mock Mode Auto-Detection

The UI automatically disables mock mode when running against real device endpoints:

- **Mock mode enabled:** `localhost` or `127.0.0.1` hosts
- **Real device mode:** Any other hostname (e.g., `192.168.4.1`, device IP)

When mock mode is active:
- OTA upload simulates progress and success/failure
- No actual backend communication occurs
- Console logs indicate mock responses

When connecting to a real device:
- Mock API is bypassed
- Real backend endpoints are called
- OTA upload streams to `/api/admin/firmware`

### Abort Button Behavior

The **Abort** button visible during OTA upload currently has **limited functionality**:

- ✅ **Client-side:** Stops the upload progress UI countdown
- ✅ **Client-side:** Resets the form to idle state
- ❌ **Backend:** Does NOT cancel an in-progress firmware write to flash
- ❌ **Backend:** Does NOT abort OTA partition updates

**Important:** Once a firmware upload has been accepted by the backend and is streaming to the OTA partition, pressing Abort will only stop the UI progress indicator. The backend will continue writing until completion or error.

**Future Enhancement:** Backend support for cancellable OTA streams requires additional work to safely abort flash writes and clean up partial OTA partitions.

### Testing OTA Upload

**Mock Mode (Local Development):**

```bash
# Start static server on localhost
npm run dev  # or http-server -p 3000

# Navigate to http://localhost:3000
# Login → Admin view → OTA Upload
# Select any .bin file
# Observe mock upload progress and completion
```

**Real Device Mode:**

```bash
# Flash firmware to ESP32C6
cd nextgen && cargo espflash flash --release --monitor

# Note device IP address from serial output
# Navigate to http://<device-ip>
# Login → Admin view → OTA Upload
# Select valid ESP32 firmware .bin file
# Upload streams to device, device reboots on success
```

**Validation Test Cases:**

1. **Invalid file size:** Upload >2MB file → expect immediate error
2. **Invalid file type:** Upload .txt or .jpg → expect immediate error
3. **Invalid magic bytes:** Upload non-firmware .bin → expect immediate error
4. **Valid firmware (mock):** Upload valid .bin on localhost → expect simulated success
5. **Valid firmware (device):** Upload on real device → expect actual OTA update + reboot

### Security Notes

- OTA endpoint requires admin authentication (bearer token)
- File validation runs **before** upload to prevent malicious payloads
- Backend performs additional signature verification (when implemented per #14)

---

## Next steps

- Add more tests for interactive flows (login + status view), and mock network endpoints if desired.
- Add a GitHub Actions workflow to run tests on PRs.
- Add Playwright E2E tests for OTA upload validation and mock behavior
