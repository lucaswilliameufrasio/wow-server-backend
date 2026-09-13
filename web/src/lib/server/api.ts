import { env } from '$env/dynamic/public';

export interface SignInResponse {
	account_id: number;
	username: string;
	email: string | null;
	access_token: string;
	refresh_token: string;
	expires_in_seconds: number;
	refresh_expires_in_seconds: number;
}

export interface AuthMeResponse {
	account_id: number;
	username: string;
	gm_level: number;
	roles: string[];
	permissions: string[];
}

export interface RefreshResponse {
	access_token: string;
	refresh_token: string;
	expires_in_seconds: number;
	refresh_expires_in_seconds: number;
}

export interface CharacterSummary {
	guid: number;
	name: string;
	race: number;
	class_id: number;
	gender: number;
	level: number;
	map: number;
	zone: number;
	online: boolean;
	money: number;
}

export interface CharacterListResponse {
	account_id: number;
	characters: CharacterSummary[];
}

export interface CharacterLocationResponse {
	guid: number;
	name: string;
	map: number;
	zone: number;
	position_x: number;
	position_y: number;
	position_z: number;
	online: boolean;
}

export interface ApiErrorBody {
	message?: string;
	error_code?: string;
}

export class BackendError extends Error {
	constructor(
		public readonly status: number,
		message: string,
		public readonly code?: string
	) {
		super(message);
		this.name = 'BackendError';
	}
}

function getApiUrl() {
	const apiUrl = env.PUBLIC_API_URL?.trim();
	if (!apiUrl) throw new Error('PUBLIC_API_URL is not configured');
	return apiUrl.replace(/\/$/, '');
}

export async function backendRequest<T>(
	path: string,
	init: RequestInit & { accessToken?: string } = {}
): Promise<T> {
	const { accessToken, ...requestInit } = init;
	const headers = new Headers(requestInit.headers);
	headers.set('content-type', 'application/json');
	if (accessToken) headers.set('authorization', `Bearer ${accessToken}`);

	const response = await fetch(`${getApiUrl()}${path}`, {
		...requestInit,
		headers
	});

	if (response.ok) {
		if (response.status === 204) return undefined as T;
		return (await response.json()) as T;
	}

	const body = (await response.json().catch(() => ({}))) as ApiErrorBody;
	throw new BackendError(
		response.status,
		body.message ?? 'Backend request failed',
		body.error_code
	);
}
