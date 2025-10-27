const { test, expect } = require('@playwright/test');

test('index loads and shows login form', async ({ page }) => {
  // Navigate to root (playwright will use the configured webServer baseURL)
  await page.goto('/');

  // Basic assertions to ensure the SPA skeleton renders
  await expect(page).toHaveTitle(/Solar Heater Controller/);
  await expect(page.locator('#login-form h1')).toHaveText('Solar Heater Login');
  await expect(page.locator('input#username')).toBeVisible();
  await expect(page.locator('input#password')).toBeVisible();
  await expect(page.locator('button[type="submit"]')).toBeVisible();
});
