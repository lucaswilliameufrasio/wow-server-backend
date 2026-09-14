import { afterEach, describe, expect, it, vi } from 'vitest';
import { BackendError, backendRequest } from './api';

describe('backendRequest', () => {
	afterEach(() => vi.restoreAllMocks());

	it('Should send JSON and bearer headers and parse a successful response', async () => {
		const fetchMock = vi
			.spyOn(globalThis, 'fetch')
			.mockResolvedValue(new Response(JSON.stringify({ ok: true }), { status: 200 }));

		await expect(
			backendRequest<{ ok: boolean }>('/v1/test', {
				method: 'POST',
				accessToken: 'access-token',
				body: JSON.stringify({ input: true })
			})
		).resolves.toEqual({ ok: true });

		const [, init] = fetchMock.mock.calls[0];
		expect(init?.method).toBe('POST');
		expect(new Headers(init?.headers).get('content-type')).toBe('application/json');
		expect(new Headers(init?.headers).get('authorization')).toBe('Bearer access-token');
	});

	it('Should return undefined for an empty successful response', async () => {
		vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 204 }));

		await expect(backendRequest('/v1/test')).resolves.toBeUndefined();
	});

	it('Should map JSON API errors to BackendError', async () => {
		vi.spyOn(globalThis, 'fetch').mockResolvedValue(
			new Response(JSON.stringify({ message: 'Denied', error_code: 'DENIED' }), { status: 403 })
		);

		const error = await backendRequest('/v1/test').catch((value) => value);
		expect(error).toBeInstanceOf(BackendError);
		expect(error).toMatchObject({ status: 403, message: 'Denied', code: 'DENIED' });
	});

	it('Should use a fallback message for malformed API errors', async () => {
		vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('not-json', { status: 500 }));

		await expect(backendRequest('/v1/test')).rejects.toMatchObject({
			status: 500,
			message: 'Backend request failed'
		});
	});
});
