import { spawn } from 'node:child_process';

const env = { ...process.env, PUBLIC_API_URL: 'http://127.0.0.1:4174' };
const children = [];

function run(command, args, options = {}) {
	const child = spawn(command, args, { stdio: 'inherit', ...options });
	children.push(child);
	return child;
}

const mock = run(process.execPath, ['e2e/mock-api.mjs']);
const build = run('pnpm', ['run', 'build'], { env });

build.on('exit', (code, signal) => {
	if (code !== 0 || signal) {
		process.exitCode = code ?? 1;
		return;
	}

	run('pnpm', ['run', 'preview', '--', '--host', '127.0.0.1'], { env });
});

function shutdown() {
	for (const child of children) child.kill('SIGTERM');
}

process.on('SIGINT', shutdown);
process.on('SIGTERM', shutdown);
mock.on('error', () => (process.exitCode = 1));
