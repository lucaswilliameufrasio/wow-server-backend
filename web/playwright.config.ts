import { defineConfig } from '@playwright/test';

export default defineConfig({
	webServer: {
		command: 'node e2e/test-server.mjs',
		port: 4173
	},
	use: { baseURL: 'http://localhost:4173' },
	testMatch: '**/*.e2e.{ts,js}'
});
