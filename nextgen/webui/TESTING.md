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

Next steps

- Add more tests for interactive flows (login + status view), and mock network endpoints if desired.
- Add a GitHub Actions workflow to run tests on PRs.
