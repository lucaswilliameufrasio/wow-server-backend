## Development

Astro 7 daemonizes preview/dev servers:

```
pnpm astro dev          # prints the running URL; survives the shell
pnpm astro dev stop     # or: status / logs
```

## Testing

Build then run the end-to-end suite (Playwright serves `dist/` itself):

```
pnpm build && pnpm test:e2e
```

The gate for any change here: **build + test:e2e green**.

## Notes

- Tailwind v4 via `@tailwindcss/vite`; styling lives in utility classes in `src/pages/index.astro`.
- Theme is an original "MMO tavern at night" look (obsidian + gold) — no game-brand
  assets or trademarks; do not copy Blizzard art, logos or iconography.
