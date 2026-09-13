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
	event.locals.session = await getCurrentAccount(event.cookies).then((account) =>
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
