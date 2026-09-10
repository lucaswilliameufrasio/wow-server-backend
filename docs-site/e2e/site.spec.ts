import { expect, test } from "@playwright/test";

const GITHUB = "https://github.com/lucaswilliameufrasio/wow-server-backend";

test.describe("wow-server-backend docs site", () => {
  test("renders SEO metadata and favicon", async ({ page }) => {
    const response = await page.goto("/");
    expect(response?.status()).toBe(200);

    await expect(page).toHaveTitle(/wow-server-backend/i);

    expect(
      await page.locator('meta[property="og:title"]').getAttribute("content"),
    ).toMatch(/wow-server-backend/i);
    expect(
      await page.locator('meta[name="description"]').getAttribute("content"),
    ).toContain("MCP");
    expect(
      await page.locator('meta[property="og:image"]').getAttribute("content"),
    ).toContain("og-image.png");

    const favicon = await page.request.get("/favicon.svg");
    expect(favicon.status()).toBe(200);

    const ogImage = await page.request.get("/og-image.png");
    expect(ogImage.status()).toBe(200);
  });

  test("Tailwind theme is actually applied (not just imported)", async ({ page }) => {
    await page.goto("/");

    const bodyBg = await page.evaluate(
      () => getComputedStyle(document.body).backgroundColor,
    );
    expect(bodyBg).toBe("rgb(18, 16, 12)");

    const navBorder = await page.evaluate(() => {
      const nav = document.querySelector("nav");
      if (!nav) return "";
      return getComputedStyle(nav).borderBottomColor;
    });
    expect(navBorder).toBe("rgb(51, 43, 32)");
  });

  test("nav links point at the repo, tools anchor and MCP docs", async ({ page }) => {
    await page.goto("/");

    await expect(
      page.locator('nav a[aria-label*="home"]'),
    ).toBeVisible();

    await expect(
      page.locator('nav a[href="#tools"]'),
    ).toBeAttached();

    const github = page.locator('nav a[href^="https://github.com/"]');
    expect(await github.count()).toBeGreaterThanOrEqual(1);
    expect(await github.first().getAttribute("href")).toBe(GITHUB);
    await expect(github.first()).toBeVisible();
  });

  test("hero contains the pitch, CTA and stat row", async ({ page }) => {
    await page.goto("/");

    const h1 = page.locator("h1");
    await expect(h1).toContainText("guild table");

    const cta = page.locator('a[href="#quick-start"]');
    await expect(cta.first()).toContainText(/raid night/i);

    await expect(page.locator("text=22 MCP tools")).toBeVisible();
  });

  test("terminal shows the wowctl ritual", async ({ page }) => {
    await page.goto("/");

    const terminal = page.locator('[aria-label="Example setup session"]');
    await expect(terminal).toBeVisible();
    await expect(terminal).toContainText("./wowctl install");
    await expect(terminal).toContainText("./wowctl setup-game");
    await expect(terminal).toContainText("create-mcp-token");
    await expect(terminal).toContainText("wowst_");
  });

  test("feature grid has three cards with the core promises", async ({ page }) => {
    await page.goto("/");

    const grid = page.locator("#tools");
    await expect(grid).toBeVisible();

    const cards = grid.locator("article");
    await expect(cards).toHaveCount(3);

    await expect(cards.nth(0)).toContainText("22");
    await expect(cards.nth(1)).toContainText("dry_run=false");
    await expect(cards.nth(2)).toContainText("audit log");
  });

  test("quick start section documents the deploy ritual", async ({ page }) => {
    await page.goto("/");

    const section = page.locator("#quick-start");
    await expect(section).toBeVisible();

    const code = section.locator("pre");
    await expect(code).toContainText("./wowctl install");
    await expect(code).toContainText("./wowctl smoke-test");

    const guideLink = section.locator('a[href*="docs/vps"]').first();
    expect(await guideLink.getAttribute("href")).toContain(
      "tree/main/docs/vps",
    );
  });

  test("MCP section explains the ai client setup", async ({ page }) => {
    await page.goto("/");

    const pre = page.locator("pre").filter({ hasText: "MCP_API_TOKEN" });
    await expect(pre).toBeVisible();
    await expect(pre).toContainText("MCP_API_URL=http://127.0.0.1:3000");
    await expect(pre).toContainText("wow-mcp");

    const docsLink = page.locator('a[href*="blob/main/docs/mcp.md"]');
    expect(await docsLink.count()).toBeGreaterThan(0);
  });

  test("legal disclaimer keeps us far from Blizzard trademarks", async ({ page }) => {
    await page.goto("/");

    const disclaimer = page.locator("text=Not affiliated with, endorsed by or associated with Blizzard");
    await expect(disclaimer.first()).toBeVisible();

    const body = await page.content();
    expect(body).not.toContain("World of Warcraft®");
    expect(body).not.toContain("Activision");
  });

  test("footer is present", async ({ page }) => {
    await page.goto("/");
    const footer = page.locator("footer");
    await expect(footer).toContainText("wow-server-backend");
  });
});

test.describe("layout sanity", () => {
  test("no horizontal overflow on desktop", async ({ page }) => {
    await page.goto("/");
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow).toBeLessThanOrEqual(1);
  });

  test("no horizontal overflow on mobile and all key sections visible", async ({ page }, testInfo) => {
    test.skip(testInfo.project.name !== "mobile", "mobile-only check");
    await page.goto("/");

    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow).toBeLessThanOrEqual(1);

    await expect(page.locator("h1")).toBeVisible();
    await expect(page.locator("#quick-start")).toBeVisible();
    await expect(page.locator("footer")).toBeVisible();
  });
});
