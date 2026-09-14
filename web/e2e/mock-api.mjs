import http from 'node:http';

const port = Number(process.env.MOCK_API_PORT ?? 4174);

const account = {
	account_id: 9001,
	username: 'demo-player',
	email: null,
	gm_level: 0,
	roles: ['player'],
	permissions: []
};

const character = {
	guid: 9001,
	name: 'Demohero',
	race: 1,
	class_id: 1,
	gender: 0,
	level: 80,
	map: 0,
	zone: 0,
	online: false,
	money: 0
};

function respond(response, status, body) {
	response.writeHead(status, { 'content-type': 'application/json' });
	response.end(body === undefined ? undefined : JSON.stringify(body));
}

function readBody(request) {
	return new Promise((resolve, reject) => {
		let body = '';
		request.setEncoding('utf8');
		request.on('data', (chunk) => (body += chunk));
		request.on('end', () => resolve(body ? JSON.parse(body) : {}));
		request.on('error', reject);
	});
}

const server = http.createServer(async (request, response) => {
	try {
		const url = new URL(request.url, `http://${request.headers.host}`);
		const authorization = request.headers.authorization;

		if (request.method === 'POST' && url.pathname === '/v1/auth/sign-in') {
			const body = await readBody(request);
			if (body.username !== account.username || body.password !== 'DemoPassword9!') {
				respond(response, 401, { message: 'Invalid credentials' });
				return;
			}
			respond(response, 200, {
				account_id: account.account_id,
				username: account.username,
				email: account.email,
				access_token: 'e2e-access-token',
				refresh_token: 'e2e-refresh-token',
				expires_in_seconds: 3600,
				refresh_expires_in_seconds: 86400
			});
			return;
		}

		if (request.method === 'POST' && url.pathname === '/v1/auth/refresh') {
			respond(response, 200, {
				access_token: 'e2e-access-token',
				refresh_token: 'e2e-refresh-token',
				expires_in_seconds: 3600,
				refresh_expires_in_seconds: 86400
			});
			return;
		}

		if (request.method === 'POST' && url.pathname === '/v1/auth/logout') {
			response.writeHead(204);
			response.end();
			return;
		}

		if (authorization !== 'Bearer e2e-access-token') {
			respond(response, 401, { message: 'Unauthorized' });
			return;
		}

		if (request.method === 'GET' && url.pathname === '/v1/auth/me') {
			respond(response, 200, account);
			return;
		}

		if (request.method === 'GET' && url.pathname === '/v1/characters') {
			respond(response, 200, { account_id: account.account_id, characters: [character] });
			return;
		}

		respond(response, 404, { message: 'Not found' });
	} catch {
		respond(response, 400, { message: 'Invalid request' });
	}
});

server.listen(port, '127.0.0.1');
