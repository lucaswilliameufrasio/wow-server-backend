import type { Handle } from '@sveltejs/kit';
import { getTextDirection } from '$lib/paraglide/runtime';
import { paraglideMiddleware } from '$lib/paraglide/server';
import { getCurrentAccount } from '$lib/server/auth';

const handleParaglide: Handle = ({ event, resolve }) =>
	paraglideMiddleware(event.request, ({ request, locale }) => {
		event.request = request;

		return resolve(event, {
			transformPageChunk: ({ html }) =>
				html
					.replace('%paraglide.lang%', locale)
					.replace('%paraglide.dir%', getTextDirection(locale))
		});
	});

const handleSession: Handle = async ({ event, resolve }) => {
	const pathname = event.url.pathname;
	const needsSession =
		pathname === '/login' ||
		pathname === '/register' ||
		pathname.startsWith('/dashboard') ||
		pathname.startsWith('/characters');

	if (!needsSession) {
		event.locals.session = null;
		return resolve(event);
	}

	event.locals.session = await getCurrentAccount(
		event.cookies,
		event.url.protocol === 'https:'
	).then((account) =>
		account
			? {
					accountId: account.account_id,
					username: account.username,
					email: null
				}
			: null
	);
	return resolve(event);
};

export const handle: Handle = ({ event, resolve }) =>
	handleParaglide({
		event,
		resolve: (localizedEvent, localizedOptions) =>
			handleSession({
				event: localizedEvent,
				resolve: (sessionEvent) => resolve(sessionEvent, localizedOptions)
			})
	});
