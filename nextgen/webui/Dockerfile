# Docker image to run Playwright web UI tests for the Solar Heater project.
# Uses the Playwright-maintained image which includes required browser dependencies.
FROM mcr.microsoft.com/playwright:latest

WORKDIR /workspace

# Copy only package files first for better caching
COPY package.json package-lock.json* ./

# Install npm deps (devDeps include @playwright/test)
RUN npm ci --no-audit --no-fund

# Copy the rest of the webui sources
COPY . .

# Ensure Playwright browsers and dependencies are installed
RUN npx playwright install --with-deps

# Default command runs the Playwright test suite
CMD ["npx", "playwright", "test", "--reporter=dot"]
