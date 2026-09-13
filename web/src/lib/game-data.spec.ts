import { describe, expect, it } from 'vitest';
import { formatMoney, getClassName, getFaction, getRaceName } from './game-data';

describe('game data labels', () => {
	it('formats character metadata', () => {
		expect(getRaceName(1)).toBe('Human');
		expect(getClassName(8)).toBe('Mage');
		expect(getFaction(2)).toBe('Horde');
	});

	it('formats copper as gold, silver and copper', () => {
		expect(formatMoney(123456)).toBe('12g 34s 56c');
	});
});
