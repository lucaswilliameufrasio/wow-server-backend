import { requireAccount } from '$lib/server/auth';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = ({ locals }) => ({ account: requireAccount(locals) });
