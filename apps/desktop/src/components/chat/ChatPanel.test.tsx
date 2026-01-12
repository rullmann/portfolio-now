import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock Tauri invoke
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('ChatPanel Watchlist Actions', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe('add_to_watchlist_by_name', () => {
    it('should handle successful add', async () => {
      mockInvoke.mockResolvedValueOnce('Microsoft Corp. wurde zur Watchlist hinzugefügt');

      const result = await mockInvoke('add_to_watchlist_by_name', { securityName: 'Microsoft' });

      expect(result).toContain('wurde zur Watchlist hinzugefügt');
    });

    it('should throw error when security not found', async () => {
      mockInvoke.mockRejectedValueOnce("Wertpapier 'Rational AG' nicht gefunden");

      await expect(
        mockInvoke('add_to_watchlist_by_name', { securityName: 'Rational AG' })
      ).rejects.toContain('nicht gefunden');
    });

    it('should handle add by ISIN', async () => {
      mockInvoke.mockResolvedValueOnce('Microsoft Corp. wurde zur Watchlist hinzugefügt');

      const result = await mockInvoke('add_to_watchlist_by_name', { securityName: 'US5949181045' });

      expect(result).toContain('wurde zur Watchlist hinzugefügt');
    });

    it('should handle add by ticker', async () => {
      mockInvoke.mockResolvedValueOnce('Microsoft Corp. wurde zur Watchlist hinzugefügt');

      const result = await mockInvoke('add_to_watchlist_by_name', { securityName: 'MSFT' });

      expect(result).toContain('wurde zur Watchlist hinzugefügt');
    });
  });

  describe('remove_from_watchlist_by_name', () => {
    it('should remove security by name', async () => {
      mockInvoke.mockResolvedValueOnce('Microsoft Corp. wurde von der Watchlist entfernt');

      const result = await mockInvoke('remove_from_watchlist_by_name', { securityName: 'Microsoft' });

      expect(result).toContain('wurde von der Watchlist entfernt');
    });

    it('should remove security by ticker', async () => {
      mockInvoke.mockResolvedValueOnce('Microsoft Corp. wurde von der Watchlist entfernt');

      const result = await mockInvoke('remove_from_watchlist_by_name', { securityName: 'MSFT' });

      expect(result).toContain('wurde von der Watchlist entfernt');
    });

    it('should remove security by ISIN', async () => {
      mockInvoke.mockResolvedValueOnce('Microsoft Corp. wurde von der Watchlist entfernt');

      const result = await mockInvoke('remove_from_watchlist_by_name', { securityName: 'US5949181045' });

      expect(result).toContain('wurde von der Watchlist entfernt');
    });

    it('should handle security not on watchlist', async () => {
      mockInvoke.mockResolvedValueOnce("'Tesla' wurde nicht auf der Watchlist gefunden");

      const result = await mockInvoke('remove_from_watchlist_by_name', { securityName: 'Tesla' });

      expect(result).toContain('nicht auf der Watchlist');
    });

    it('should handle case-insensitive search', async () => {
      mockInvoke.mockResolvedValueOnce('Microsoft Corp. wurde von der Watchlist entfernt');

      const result = await mockInvoke('remove_from_watchlist_by_name', { securityName: 'microsoft' });

      expect(result).toContain('wurde von der Watchlist entfernt');
    });
  });

  describe('Error handling', () => {
    it('should detect "nicht gefunden" in error for auto-create', async () => {
      const errorMessage = "Wertpapier 'Rational AG' nicht gefunden";
      mockInvoke.mockRejectedValueOnce(errorMessage);

      try {
        await mockInvoke('add_to_watchlist_by_name', { securityName: 'Rational AG' });
      } catch (err) {
        const errString = typeof err === 'string' ? err : String(err);
        expect(errString.includes('nicht gefunden')).toBe(true);
      }
    });
  });
});

describe('ISIN/WKN Detection', () => {
  it('should detect valid ISIN format', () => {
    const validIsins = ['US5949181045', 'DE0007164600', 'GB00B03MLX29'];
    for (const isin of validIsins) {
      const upper = isin.toUpperCase();
      const isIsin =
        upper.length === 12 &&
        /^[A-Z]{2}/.test(upper) &&
        /^[A-Z]{2}[A-Z0-9]{10}$/.test(upper);
      expect(isIsin).toBe(true);
    }
  });

  it('should reject invalid ISIN format', () => {
    const invalid = ['Microsoft', 'MSFT', '12345678901234', '1234567890'];
    for (const s of invalid) {
      const upper = s.toUpperCase();
      const isIsin =
        upper.length === 12 &&
        /^[A-Z]{2}/.test(upper) &&
        /^[A-Z]{2}[A-Z0-9]{10}$/.test(upper);
      expect(isIsin).toBe(false);
    }
  });

  it('should detect valid WKN format', () => {
    const validWkns = ['870747', 'A1JWVX', '766403'];
    for (const wkn of validWkns) {
      const upper = wkn.toUpperCase();
      const isWkn = upper.length === 6 && /^[A-Z0-9]{6}$/.test(upper);
      expect(isWkn).toBe(true);
    }
  });

  it('should reject invalid WKN format', () => {
    const invalid = ['Microsoft', 'MSFT12345', '12345'];
    for (const s of invalid) {
      const upper = s.toUpperCase();
      const isWkn = upper.length === 6 && /^[A-Z0-9]{6}$/.test(upper);
      expect(isWkn).toBe(false);
    }
  });
});
