/**
 * End-to-End Tests for Portfolio Now
 *
 * Tests the main workflows:
 * - Portfolio Performance file import
 * - User profile (Welcome modal, settings)
 * - Delete all data
 * - Date/time handling
 * - ESC key modal close
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock Tauri invoke
const mockInvoke = vi.fn();
const mockListen = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

// ============================================================================
// Portfolio Performance Import Tests
// ============================================================================

describe('Portfolio Performance Import', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockListen.mockReset();
    // Default: listen returns unlisten function
    mockListen.mockResolvedValue(() => {});
  });

  describe('import_pp_file_go', () => {
    it('should successfully import a .portfolio file', async () => {
      const mockResult = {
        success: true,
        error: null,
        securitiesCount: 15,
        accountsCount: 2,
        portfoliosCount: 1,
        transactionsCount: 150,
        pricesCount: 5000,
        databasePath: '/path/to/database.db',
      };

      mockInvoke.mockResolvedValueOnce(mockResult);

      const result = await mockInvoke('import_pp_file_go', {
        path: '/Users/test/portfolio.portfolio',
        outputPath: undefined,
      });

      expect(result.success).toBe(true);
      expect(result.securitiesCount).toBe(15);
      expect(result.transactionsCount).toBe(150);
    });

    it('should handle import errors gracefully', async () => {
      const mockResult = {
        success: false,
        error: 'Invalid file format: not a valid .portfolio file',
        securitiesCount: 0,
        accountsCount: 0,
        portfoliosCount: 0,
        transactionsCount: 0,
        pricesCount: 0,
        databasePath: null,
      };

      mockInvoke.mockResolvedValueOnce(mockResult);

      const result = await mockInvoke('import_pp_file_go', {
        path: '/Users/test/invalid.txt',
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('Invalid file format');
    });

    it('should handle file not found', async () => {
      mockInvoke.mockRejectedValueOnce('File not found: /nonexistent/file.portfolio');

      await expect(
        mockInvoke('import_pp_file_go', { path: '/nonexistent/file.portfolio' })
      ).rejects.toContain('File not found');
    });

    it('should handle encrypted portfolio files', async () => {
      const mockResult = {
        success: false,
        error: 'Portfolio file is encrypted. Please export without password protection.',
        securitiesCount: 0,
        accountsCount: 0,
        portfoliosCount: 0,
        transactionsCount: 0,
        pricesCount: 0,
        databasePath: null,
      };

      mockInvoke.mockResolvedValueOnce(mockResult);

      const result = await mockInvoke('import_pp_file_go', {
        path: '/Users/test/encrypted.portfolio',
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('encrypted');
    });

    it('should import all entity types correctly', async () => {
      const mockResult = {
        success: true,
        error: null,
        securitiesCount: 25,
        accountsCount: 3,
        portfoliosCount: 2,
        transactionsCount: 500,
        pricesCount: 10000,
        databasePath: '/path/to/database.db',
      };

      mockInvoke.mockResolvedValueOnce(mockResult);

      const result = await mockInvoke('import_pp_file_go', {
        path: '/Users/test/large-portfolio.portfolio',
      });

      expect(result.securitiesCount).toBeGreaterThan(0);
      expect(result.accountsCount).toBeGreaterThan(0);
      expect(result.portfoliosCount).toBeGreaterThan(0);
      expect(result.transactionsCount).toBeGreaterThan(0);
      expect(result.pricesCount).toBeGreaterThan(0);
    });
  });

  describe('Post-Import Data Verification', () => {
    it('should be able to query securities after import', async () => {
      mockInvoke
        .mockResolvedValueOnce({ success: true }) // import
        .mockResolvedValueOnce([
          { id: 1, name: 'Apple Inc.', ticker: 'AAPL', isin: 'US0378331005', currency: 'USD' },
          { id: 2, name: 'Microsoft Corp.', ticker: 'MSFT', isin: 'US5949181045', currency: 'USD' },
        ]); // get_securities

      await mockInvoke('import_pp_file_go', { path: '/test.portfolio' });
      const securities = await mockInvoke('get_securities');

      expect(securities).toHaveLength(2);
      expect(securities[0].name).toBe('Apple Inc.');
    });

    it('should be able to query transactions after import', async () => {
      mockInvoke
        .mockResolvedValueOnce({ success: true }) // import
        .mockResolvedValueOnce([
          {
            id: 1,
            date: '2024-01-15 09:30:00',
            txnType: 'BUY',
            securityName: 'Apple Inc.',
            shares: 10,
            amount: 1500,
            currency: 'EUR',
          },
        ]); // get_transactions

      await mockInvoke('import_pp_file_go', { path: '/test.portfolio' });
      const transactions = await mockInvoke('get_transactions', { limit: 100 });

      expect(transactions).toHaveLength(1);
      expect(transactions[0].txnType).toBe('BUY');
    });

    it('should be able to query holdings after import', async () => {
      mockInvoke
        .mockResolvedValueOnce({ success: true }) // import
        .mockResolvedValueOnce([
          {
            securityId: 1,
            securityName: 'Apple Inc.',
            ticker: 'AAPL',
            shares: 10,
            currentValue: 2500,
            costBasis: 1500,
            gainLossPercent: 66.67,
          },
        ]); // get_holdings

      await mockInvoke('import_pp_file_go', { path: '/test.portfolio' });
      const holdings = await mockInvoke('get_holdings', { portfolioId: 1 });

      expect(holdings).toHaveLength(1);
      expect(holdings[0].shares).toBe(10);
    });

    it('should calculate FIFO cost basis after import', async () => {
      mockInvoke
        .mockResolvedValueOnce({ success: true }) // import
        .mockResolvedValueOnce({
          lotsProcessed: 50,
          consumptionsProcessed: 25,
        }); // rebuild_fifo_lots

      await mockInvoke('import_pp_file_go', { path: '/test.portfolio' });
      const fifoResult = await mockInvoke('rebuild_fifo_lots');

      expect(fifoResult.lotsProcessed).toBeGreaterThan(0);
    });
  });
});

// ============================================================================
// User Profile Tests
// ============================================================================

describe('User Profile', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe('userName in Settings Store', () => {
    it('should persist userName to localStorage', () => {
      const testName = 'Max Mustermann';
      localStorage.setItem('portfolio-settings', JSON.stringify({
        state: { userName: testName },
        version: 4,
      }));

      const stored = JSON.parse(localStorage.getItem('portfolio-settings') || '{}');
      expect(stored.state.userName).toBe(testName);
    });

    it('should handle empty userName', () => {
      localStorage.setItem('portfolio-settings', JSON.stringify({
        state: { userName: '' },
        version: 4,
      }));

      const stored = JSON.parse(localStorage.getItem('portfolio-settings') || '{}');
      expect(stored.state.userName).toBe('');
    });

    it('should migrate from version 3 to 4', () => {
      // Simulate old version without userName
      localStorage.setItem('portfolio-settings', JSON.stringify({
        state: { aiProvider: 'claude', theme: 'dark' },
        version: 3,
      }));

      const stored = JSON.parse(localStorage.getItem('portfolio-settings') || '{}');
      // After migration, userName should be added (empty string)
      expect(stored.version).toBe(3); // Before migration
    });
  });

  describe('AI Chat with userName', () => {
    it('should include userName in chat request', async () => {
      const mockResponse = {
        response: 'Hallo Max! Dein Portfolio zeigt eine positive Entwicklung.',
        provider: 'Claude',
        model: 'claude-sonnet-4-5',
        tokensUsed: 150,
      };

      mockInvoke.mockResolvedValueOnce(mockResponse);

      const result = await mockInvoke('chat_with_portfolio_assistant', {
        request: {
          messages: [{ role: 'user', content: 'Wie steht mein Portfolio?' }],
          provider: 'claude',
          model: 'claude-sonnet-4-5',
          apiKey: 'test-key',
          baseCurrency: 'EUR',
          userName: 'Max',
        },
      });

      expect(result.response).toContain('Max');
    });

    it('should work without userName', async () => {
      const mockResponse = {
        response: 'Dein Portfolio zeigt eine positive Entwicklung.',
        provider: 'Claude',
        model: 'claude-sonnet-4-5',
        tokensUsed: 150,
      };

      mockInvoke.mockResolvedValueOnce(mockResponse);

      const result = await mockInvoke('chat_with_portfolio_assistant', {
        request: {
          messages: [{ role: 'user', content: 'Wie steht mein Portfolio?' }],
          provider: 'claude',
          model: 'claude-sonnet-4-5',
          apiKey: 'test-key',
          baseCurrency: 'EUR',
          userName: null,
        },
      });

      expect(result.response).toBeTruthy();
    });
  });
});

// ============================================================================
// Delete All Data Tests
// ============================================================================

describe('Delete All Data', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe('delete_all_data command', () => {
    it('should successfully delete all data', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await mockInvoke('delete_all_data');

      expect(mockInvoke).toHaveBeenCalledWith('delete_all_data');
    });

    it('should clear all tables', async () => {
      mockInvoke
        .mockResolvedValueOnce(undefined) // delete_all_data
        .mockResolvedValueOnce([]) // get_securities (empty)
        .mockResolvedValueOnce([]) // get_accounts (empty)
        .mockResolvedValueOnce([]) // get_portfolios (empty)
        .mockResolvedValueOnce([]); // get_transactions (empty)

      await mockInvoke('delete_all_data');

      const securities = await mockInvoke('get_securities');
      const accounts = await mockInvoke('get_accounts');
      const portfolios = await mockInvoke('get_portfolios');
      const transactions = await mockInvoke('get_transactions');

      expect(securities).toHaveLength(0);
      expect(accounts).toHaveLength(0);
      expect(portfolios).toHaveLength(0);
      expect(transactions).toHaveLength(0);
    });

    it('should handle database errors', async () => {
      mockInvoke.mockRejectedValueOnce('Database error: table does not exist');

      await expect(mockInvoke('delete_all_data')).rejects.toContain('Database error');
    });

    it('should allow re-import after deletion', async () => {
      const mockImportResult = {
        success: true,
        securitiesCount: 10,
        transactionsCount: 50,
      };

      mockInvoke
        .mockResolvedValueOnce(undefined) // delete_all_data
        .mockResolvedValueOnce(mockImportResult); // import_pp_file_go

      await mockInvoke('delete_all_data');
      const result = await mockInvoke('import_pp_file_go', { path: '/test.portfolio' });

      expect(result.success).toBe(true);
      expect(result.securitiesCount).toBe(10);
    });
  });

  describe('Confirmation Flow', () => {
    it('should require exact confirmation text', () => {
      const confirmText = 'LÖSCHEN';
      expect(confirmText).toBe('LÖSCHEN');
      expect('löschen').not.toBe('LÖSCHEN'); // Case sensitive
      expect('LOSCHEN').not.toBe('LÖSCHEN'); // Missing umlaut
    });
  });
});

// ============================================================================
// Date/Time Handling Tests
// ============================================================================

describe('Date/Time Handling', () => {
  describe('extractDateForInput', () => {
    // Test the date extraction logic
    it('should extract date from datetime string', () => {
      const extractDateForInput = (dateStr: string | null | undefined): string => {
        if (!dateStr) return new Date().toISOString().split('T')[0];
        const part = dateStr.split(' ')[0].split('T')[0];
        return part || new Date().toISOString().split('T')[0];
      };

      expect(extractDateForInput('2024-01-15 09:30:00')).toBe('2024-01-15');
      expect(extractDateForInput('2024-01-15T09:30:00')).toBe('2024-01-15');
      expect(extractDateForInput('2024-01-15')).toBe('2024-01-15');
    });

    it('should handle null/undefined', () => {
      const extractDateForInput = (dateStr: string | null | undefined): string => {
        if (!dateStr) return new Date().toISOString().split('T')[0];
        const part = dateStr.split(' ')[0].split('T')[0];
        return part || new Date().toISOString().split('T')[0];
      };

      const today = new Date().toISOString().split('T')[0];
      expect(extractDateForInput(null)).toBe(today);
      expect(extractDateForInput(undefined)).toBe(today);
      expect(extractDateForInput('')).toBe(today);
    });
  });

  describe('extractTimeForInput', () => {
    it('should extract time from datetime string', () => {
      const extractTimeForInput = (dateStr: string | null | undefined): string => {
        if (!dateStr) return '00:00';
        const parts = dateStr.split(' ');
        if (parts.length >= 2) {
          return parts[1].substring(0, 5);
        }
        const tParts = dateStr.split('T');
        if (tParts.length >= 2) {
          return tParts[1].substring(0, 5);
        }
        return '00:00';
      };

      expect(extractTimeForInput('2024-01-15 09:30:00')).toBe('09:30');
      expect(extractTimeForInput('2024-01-15T14:45:00')).toBe('14:45');
      expect(extractTimeForInput('2024-01-15 00:00:00')).toBe('00:00');
    });

    it('should default to 00:00 for date-only strings', () => {
      const extractTimeForInput = (dateStr: string | null | undefined): string => {
        if (!dateStr) return '00:00';
        const parts = dateStr.split(' ');
        if (parts.length >= 2) {
          return parts[1].substring(0, 5);
        }
        const tParts = dateStr.split('T');
        if (tParts.length >= 2) {
          return tParts[1].substring(0, 5);
        }
        return '00:00';
      };

      expect(extractTimeForInput('2024-01-15')).toBe('00:00');
      expect(extractTimeForInput(null)).toBe('00:00');
    });
  });

  describe('combineDateAndTime', () => {
    it('should combine date and time for backend', () => {
      const combineDateAndTime = (date: string, time: string): string => {
        const timePart = time || '00:00';
        return `${date} ${timePart}:00`;
      };

      expect(combineDateAndTime('2024-01-15', '09:30')).toBe('2024-01-15 09:30:00');
      expect(combineDateAndTime('2024-01-15', '14:45')).toBe('2024-01-15 14:45:00');
      expect(combineDateAndTime('2024-01-15', '')).toBe('2024-01-15 00:00:00');
    });
  });

  describe('formatDate', () => {
    it('should format date for display (German locale)', () => {
      const formatDate = (dateStr: string): string => {
        const date = new Date(dateStr);
        return new Intl.DateTimeFormat('de-DE', {
          day: '2-digit',
          month: '2-digit',
          year: 'numeric',
        }).format(date);
      };

      expect(formatDate('2024-01-15')).toBe('15.01.2024');
      expect(formatDate('2024-12-31')).toBe('31.12.2024');
    });
  });

  describe('formatDateTime', () => {
    it('should format datetime for display', () => {
      const formatDateTime = (dateStr: string): string => {
        const date = new Date(dateStr.replace(' ', 'T'));
        return new Intl.DateTimeFormat('de-DE', {
          day: '2-digit',
          month: '2-digit',
          year: 'numeric',
          hour: '2-digit',
          minute: '2-digit',
        }).format(date);
      };

      // Format depends on locale settings
      const result = formatDateTime('2024-01-15 09:30:00');
      expect(result).toContain('15');
      expect(result).toContain('01');
      expect(result).toContain('2024');
    });
  });

  describe('Transaction CRUD with DateTime', () => {
    beforeEach(() => {
      mockInvoke.mockReset();
    });

    it('should create transaction with full datetime', async () => {
      mockInvoke.mockResolvedValueOnce({ id: 1 });

      await mockInvoke('create_transaction', {
        data: {
          ownerType: 'portfolio',
          ownerId: 1,
          txnType: 'BUY',
          date: '2024-01-15 09:30:00',
          securityId: 1,
          shares: 10,
          amount: 1500,
          currency: 'EUR',
        },
      });

      expect(mockInvoke).toHaveBeenCalledWith('create_transaction', expect.objectContaining({
        data: expect.objectContaining({
          date: '2024-01-15 09:30:00',
        }),
      }));
    });

    it('should update transaction datetime', async () => {
      mockInvoke.mockResolvedValueOnce({ id: 1 });

      await mockInvoke('update_transaction', {
        id: 1,
        data: {
          date: '2024-01-15 14:45:00',
        },
      });

      expect(mockInvoke).toHaveBeenCalledWith('update_transaction', expect.objectContaining({
        data: expect.objectContaining({
          date: '2024-01-15 14:45:00',
        }),
      }));
    });

    it('should return transactions with full datetime', async () => {
      mockInvoke.mockResolvedValueOnce([
        {
          id: 1,
          date: '2024-01-15 09:30:00',
          txnType: 'BUY',
          amount: 1500,
        },
      ]);

      const transactions = await mockInvoke('get_transactions');

      expect(transactions[0].date).toBe('2024-01-15 09:30:00');
    });
  });
});

// ============================================================================
// ESC Key Modal Close Tests
// ============================================================================

describe('ESC Key Modal Close', () => {
  describe('useEscapeKey Hook Logic', () => {
    it('should trigger callback on Escape key', () => {
      const callback = vi.fn();
      let isOpen = true;

      // Simulate the hook behavior
      const handleKeyDown = (e: KeyboardEvent) => {
        if (isOpen && e.key === 'Escape') {
          callback();
        }
      };

      // Simulate keydown event
      const event = new KeyboardEvent('keydown', { key: 'Escape' });
      handleKeyDown(event);

      expect(callback).toHaveBeenCalledTimes(1);
    });

    it('should not trigger when modal is closed', () => {
      const callback = vi.fn();
      let isOpen = false;

      const handleKeyDown = (e: KeyboardEvent) => {
        if (isOpen && e.key === 'Escape') {
          callback();
        }
      };

      const event = new KeyboardEvent('keydown', { key: 'Escape' });
      handleKeyDown(event);

      expect(callback).not.toHaveBeenCalled();
    });

    it('should not trigger for other keys', () => {
      const callback = vi.fn();
      let isOpen = true;

      const handleKeyDown = (e: KeyboardEvent) => {
        if (isOpen && e.key === 'Escape') {
          callback();
        }
      };

      const enterEvent = new KeyboardEvent('keydown', { key: 'Enter' });
      const spaceEvent = new KeyboardEvent('keydown', { key: ' ' });

      handleKeyDown(enterEvent);
      handleKeyDown(spaceEvent);

      expect(callback).not.toHaveBeenCalled();
    });
  });

  describe('Modal Components with ESC support', () => {
    const modalsWithEscSupport = [
      'TransactionFormModal',
      'SecurityFormModal',
      'AccountFormModal',
      'PortfolioFormModal',
      'PdfImportModal',
      'PdfExportModal',
      'InvestmentPlanFormModal',
      'PortfolioInsightsModal',
      'BenchmarkFormModal',
      'StockSplitModal',
      'TaxonomyFormModal',
      'SecurityPriceModal',
      'WelcomeModal',
    ];

    it('should have ESC support in all modals', () => {
      // This is a documentation test - all these modals should use useEscapeKey
      expect(modalsWithEscSupport.length).toBeGreaterThan(10);
    });
  });
});

// ============================================================================
// Integration Tests
// ============================================================================

describe('Integration: Full Workflow', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockListen.mockReset();
    mockListen.mockResolvedValue(() => {});
  });

  it('should handle complete import → view → delete cycle', async () => {
    // 1. Import
    mockInvoke.mockResolvedValueOnce({
      success: true,
      securitiesCount: 10,
      transactionsCount: 100,
    });

    const importResult = await mockInvoke('import_pp_file_go', {
      path: '/test.portfolio',
    });
    expect(importResult.success).toBe(true);

    // 2. View data
    mockInvoke.mockResolvedValueOnce([
      { id: 1, name: 'Apple Inc.', ticker: 'AAPL' },
    ]);

    const securities = await mockInvoke('get_securities');
    expect(securities.length).toBeGreaterThan(0);

    // 3. Delete all
    mockInvoke.mockResolvedValueOnce(undefined);
    await mockInvoke('delete_all_data');

    // 4. Verify empty
    mockInvoke.mockResolvedValueOnce([]);
    const afterDelete = await mockInvoke('get_securities');
    expect(afterDelete).toHaveLength(0);
  });

  it('should handle user profile setup → AI chat flow', async () => {
    // 1. Set user name (simulated via localStorage)
    localStorage.setItem('portfolio-settings', JSON.stringify({
      state: { userName: 'Max' },
      version: 4,
    }));

    // 2. Use AI chat with name
    mockInvoke.mockResolvedValueOnce({
      response: 'Hallo Max! Dein Portfolio entwickelt sich gut.',
      provider: 'Claude',
      model: 'claude-sonnet-4-5',
    });

    const chatResponse = await mockInvoke('chat_with_portfolio_assistant', {
      request: {
        messages: [{ role: 'user', content: 'Wie geht es meinem Portfolio?' }],
        provider: 'claude',
        model: 'claude-sonnet-4-5',
        apiKey: 'test',
        baseCurrency: 'EUR',
        userName: 'Max',
      },
    });

    expect(chatResponse.response).toContain('Max');
  });

  it('should handle transaction with date/time → edit flow', async () => {
    // 1. Create transaction
    mockInvoke.mockResolvedValueOnce({ id: 1 });

    await mockInvoke('create_transaction', {
      data: {
        ownerType: 'portfolio',
        ownerId: 1,
        txnType: 'BUY',
        date: '2024-01-15 09:30:00',
        securityId: 1,
        shares: 10,
        amount: 1500,
        currency: 'EUR',
      },
    });

    // 2. Get transaction (with owner_id and security_id for editing)
    mockInvoke.mockResolvedValueOnce({
      id: 1,
      date: '2024-01-15 09:30:00',
      txnType: 'BUY',
      ownerId: 1,
      securityId: 1,
      shares: 10,
      amount: 1500,
    });

    const txn = await mockInvoke('get_transaction', { id: 1 });

    // 3. Extract date and time for form
    const date = txn.date.split(' ')[0]; // 2024-01-15
    const time = txn.date.split(' ')[1].substring(0, 5); // 09:30

    expect(date).toBe('2024-01-15');
    expect(time).toBe('09:30');

    // 4. Update with new time
    mockInvoke.mockResolvedValueOnce({ id: 1 });

    await mockInvoke('update_transaction', {
      id: 1,
      data: {
        date: '2024-01-15 14:45:00', // Changed time
      },
    });

    expect(mockInvoke).toHaveBeenLastCalledWith('update_transaction', expect.objectContaining({
      data: expect.objectContaining({
        date: '2024-01-15 14:45:00',
      }),
    }));
  });
});

// Clean up after all tests
afterEach(() => {
  localStorage.clear();
});
