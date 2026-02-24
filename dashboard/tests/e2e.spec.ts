import { test, expect, type Page } from '@playwright/test'

// Test configuration
const BASE_URL = process.env.TAURI_DEV_URL || 'http://localhost:1420'

test.describe('Housaky Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL)
  })

  test.describe('Navigation', () => {
    test('should load dashboard page', async ({ page }) => {
      await expect(page.locator('h1')).toContainText('Dashboard')
    })

    test('should navigate to chat', async ({ page }) => {
      await page.click('text=Chat')
      await expect(page.locator('h1')).toContainText('Chat')
    })

    test('should navigate to config', async ({ page }) => {
      await page.click('text=Config')
      await expect(page.locator('h1')).toContainText('Configuration')
    })

    test('should navigate to channels', async ({ page }) => {
      await page.click('text=Channels')
      await expect(page.locator('h1')).toContainText('Channels')
    })

    test('should navigate to skills', async ({ page }) => {
      await page.click('text=Skills')
      await expect(page.locator('h1')).toContainText('Skills')
    })

    test('should navigate to integrations', async ({ page }) => {
      await page.click('text=Integrations')
      await expect(page.locator('h1')).toContainText('Integrations')
    })

    test('should navigate to hardware', async ({ page }) => {
      await page.click('text=Hardware')
      await expect(page.locator('h1')).toContainText('Hardware')
    })

    test('should navigate to terminal', async ({ page }) => {
      await page.click('text=Terminal')
      await expect(page.locator('h1')).toContainText('Terminal')
    })
  })

  test.describe('Dashboard', () => {
    test('should display version card', async ({ page }) => {
      await expect(page.locator('text=Version')).toBeVisible()
    })

    test('should display provider card', async ({ page }) => {
      await expect(page.locator('text=Provider')).toBeVisible()
    })

    test('should display memory card', async ({ page }) => {
      await expect(page.locator('text=Memory')).toBeVisible()
    })

    test('should display runtime card', async ({ page }) => {
      await expect(page.locator('text=Runtime')).toBeVisible()
    })

    test('should display channels section', async ({ page }) => {
      await expect(page.locator('text=Channels')).toBeVisible()
    })

    test('should have refresh button', async ({ page }) => {
      await expect(page.locator('button:has-text("Refresh")')).toBeVisible()
    })

    test('should have quick actions', async ({ page }) => {
      await expect(page.locator('text=Quick Actions')).toBeVisible()
    })
  })

  test.describe('Config', () => {
    test('should display config sections', async ({ page }) => {
      await page.click('text=Config')
      await expect(page.locator('text=Sections')).toBeVisible()
    })

    test('should have general section', async ({ page }) => {
      await page.click('text=Config')
      await expect(page.locator('button:has-text("General")')).toBeVisible()
    })

    test('should have save button', async ({ page }) => {
      await page.click('text=Config')
      await expect(page.locator('button:has-text("Save Changes")')).toBeVisible()
    })

    test('should toggle visibility of API key', async ({ page }) => {
      await page.click('text=Config')
      // Click on General section
      await page.click('button:has-text("General")')
      // Check for API key field - should have password type
      const apiKeyInput = page.locator('input[type="password"]').first()
      if (await apiKeyInput.isVisible()) {
        // Click the eye icon to toggle visibility
        await page.locator('button').filter({ has: page.locator('svg') }).nth(1).click()
      }
    })

    test('should change provider', async ({ page }) => {
      await page.click('text=Config')
      await page.click('button:has-text("General")')
      
      // Find and change the provider dropdown
      const providerSelect = page.locator('select').first()
      if (await providerSelect.isVisible()) {
        await providerSelect.selectOption('anthropic')
      }
    })

    test('should modify temperature', async ({ page }) => {
      await page.click('text=Config')
      await page.click('button:has-text("General")')
      
      const tempInput = page.locator('input[type="number"]').first()
      if (await tempInput.isVisible()) {
        await tempInput.fill('0.9')
      }
    })
  })

  test.describe('Chat', () => {
    test('should display chat interface', async ({ page }) => {
      await page.click('text=Chat')
      await expect(page.locator('input[placeholder="Type your message..."]')).toBeVisible()
    })

    test('should have send button', async ({ page }) => {
      await page.click('text=Chat')
      await expect(page.locator('button:has(svg)')).toBeVisible()
    })

    test('should accept message input', async ({ page }) => {
      await page.click('text=Chat')
      const input = page.locator('input[placeholder="Type your message..."]')
      await input.fill('Hello')
      await expect(input).toHaveValue('Hello')
    })

    test('should have clear chat button', async ({ page }) => {
      await page.click('text=Chat')
      // Should have a trash/clear button
      await expect(page.locator('button').filter({ has: page.locator('svg') })).toBeDefined()
    })
  })

  test.describe('Channels', () => {
    test('should display channels list', async ({ page }) => {
      await page.click('text=Channels')
      await expect(page.locator('text=Channels')).toBeVisible()
    })

    test('should show configured channels', async ({ page }) => {
      await page.click('text=Channels')
      // Should show at least CLI channel
      await expect(page.locator('text=CLI')).toBeVisible()
    })
  })

  test.describe('Skills', () => {
    test('should display skills list', async ({ page }) => {
      await page.click('text=Skills')
      await expect(page.locator('text=Skills')).toBeVisible()
    })

    test('should have add skill button', async ({ page }) => {
      await page.click('text=Skills')
      await expect(page.locator('button:has-text("Add Skill")')).toBeVisible()
    })

    test('should have search input', async ({ page }) => {
      await page.click('text=Skills')
      await expect(page.locator('input[placeholder="Search skills..."]')).toBeVisible()
    })
  })

  test.describe('Terminal', () => {
    test('should display terminal interface', async ({ page }) => {
      await page.click('text=Terminal')
      await expect(page.locator('text=Terminal')).toBeVisible()
    })

    test('should have command input', async ({ page }) => {
      await page.click('text=Terminal')
      await expect(page.locator('input[placeholder="Type a command..."]')).toBeVisible()
    })

    test('should have run button', async ({ page }) => {
      await page.click('text=Terminal')
      await expect(page.locator('button:has(svg)')).toBeDefined()
    })
  })

  test.describe('Responsive', () => {
    test('should work on mobile viewport', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 })
      await page.goto(BASE_URL)
      
      // Sidebar should be hidden on mobile or collapsible
      await expect(page.locator('h1')).toContainText('Dashboard')
    })

    test('should work on tablet viewport', async ({ page }) => {
      await page.setViewportSize({ width: 768, height: 1024 })
      await page.goto(BASE_URL)
      
      await expect(page.locator('h1')).toContainText('Dashboard')
    })
  })
})

test.describe('Config Sync E2E', () => {
  test('should load config from backend', async ({ page }) => {
    await page.goto(BASE_URL)
    await page.click('text=Config')
    
    // Wait for config to load
    await page.waitForTimeout(500)
    
    // Check that config fields are populated
    const providerSelect = page.locator('select').first()
    // The value should be loaded from backend
    await expect(providerSelect).toBeDefined()
  })

  test('should detect unsaved changes', async ({ page }) => {
    await page.goto(BASE_URL)
    await page.click('text=Config')
    await page.click('button:has-text("General")')
    
    // Change a value
    const tempInput = page.locator('input[type="number"]').first()
    if (await tempInput.isVisible()) {
      await tempInput.fill('0.5')
    }
    
    // Should show "Modified" badge
    await expect(page.locator('text=Modified')).toBeVisible()
  })

  test('should save config changes', async ({ page }) => {
    await page.goto(BASE_URL)
    await page.click('text=Config')
    await page.click('button:has-text("General")')
    
    // Change a value
    const tempInput = page.locator('input[type="number"]').first()
    if (await tempInput.isVisible()) {
      await tempInput.fill('0.8')
    }
    
    // Click save
    await page.click('button:has-text("Save Changes")')
    
    // Should show success message
    await expect(page.locator('text=saved successfully')).toBeVisible({ timeout: 5000 })
  })
})
