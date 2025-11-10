const { test, expect } = require('@playwright/test');
const path = require('path');

const dummyFirmwarePath = path.join(__dirname, '..', 'dummy.bin');

test('index loads and shows login form', async ({ page }) => {
  // Navigate to root (playwright will use the configured webServer baseURL)
  await page.goto('/');

  // Basic assertions to ensure the SPA skeleton renders
  await expect(page).toHaveTitle(/Solar Heater Controller/);
  await expect(page.locator('#login-form h1')).toHaveText('Solar Heater Login');
  await expect(page.locator('input#username')).toBeVisible();
  await expect(page.locator('input#password')).toBeVisible();
  await expect(page.locator('#login-form button[type="submit"]')).toBeVisible();
});

test.describe('Firmware OTA workflow', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.fill('#username', 'admin');
    await page.fill('#password', 'admin');
    await page.click('#login-form button[type="submit"]');

    await expect(page.locator('#app-shell')).toBeVisible();
    await page.click('button[data-view="admin"]');
    await expect(page.locator('#admin-view')).toBeVisible();
  });

  test('shows validation error when file missing', async ({ page }) => {
  const message = page.locator('#firmware-message');
    await expect(message).toBeHidden();

  // Remove native required attribute so the form submission reaches our JS validation.
  await page.locator('#firmware-file').evaluate((node) => node.removeAttribute('required'));

  await page.click('#firmware-form button[type="submit"]');

    await expect(message).toBeVisible();
    await expect(message).toHaveText('Select a firmware (.bin) file before uploading.');
    await expect(page.locator('#firmware-file')).toBeEnabled();
  });

  test('successful upload starts countdown and abort resets UI', async ({ page }) => {
    const message = page.locator('#firmware-message');
    const abortButton = page.locator('#firmware-abort');
    const fileInput = page.locator('#firmware-file');

    await expect(abortButton).toBeHidden();

    await page.setInputFiles('#firmware-file', dummyFirmwarePath);
    await page.click('#firmware-form button[type="submit"]');

    await expect(message).toBeVisible();
    await expect(message).toContainText('Controller rebooting in');
    await expect(fileInput).toBeDisabled();
    await expect(abortButton).toBeVisible();

    await page.click('#firmware-abort');

    await expect(abortButton).toBeHidden();
    await expect(fileInput).toBeEnabled();
    await expect(message).toContainText('Reboot countdown cancelled.');
  });
});
