import { describe, it, expect } from 'vitest';
import { greet } from './greet';

describe('greet', () => {
	it('Should return a greeting', () => {
		expect(greet('Svelte')).toBe('Hello, Svelte!');
	});
});
