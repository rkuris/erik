# Contributing

Developer environment requirements (devcontainer + CI):

- Node.js: 18.x (LTS)
- npm: bundled with Node.js
- Rust: nightly (project uses ESP toolchain)

Notes:
- The devcontainer includes Node.js 18.x so `cd nextgen/webui && npm ci` should succeed inside the container.
- The web UI Playwright tests require additional browser dependencies. The devcontainer attempts to run `npx playwright install --with-deps` during `postCreateCommand` as a best-effort step. If Playwright browser installation fails in the container, use the Playwright Docker image provided in `nextgen/webui/Dockerfile` to run tests.

CI:
- GitHub Actions use `actions/setup-node@v4` with `node-version: '18'` for webui jobs; keep this in sync with the devcontainer Node version.

If you update the Node version used in CI, also update `.devcontainer/Dockerfile` and this file.
