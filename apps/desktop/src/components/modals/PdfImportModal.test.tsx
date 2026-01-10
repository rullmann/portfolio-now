/**
 * End-to-End Tests for PDF Import Modal
 * Tests the full flow from file selection to import completion
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { PdfImportModal } from './PdfImportModal';

// Mock Tauri dialog plugin
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

// Mock the API module
vi.mock('../../lib/api', () => ({
  getSupportedBanks: vi.fn(),
  previewPdfImport: vi.fn(),
  importPdfTransactions: vi.fn(),
  getPortfolios: vi.fn(),
  getAccounts: vi.fn(),
}));

import { open } from '@tauri-apps/plugin-dialog';
import {
  getSupportedBanks,
  previewPdfImport,
  importPdfTransactions,
  getPortfolios,
  getAccounts,
} from '../../lib/api';

const mockBanks = [
  { id: 'dkb', name: 'DKB', description: 'Deutsche Kreditbank' },
  { id: 'ing', name: 'ING', description: 'ING-DiBa' },
];

const mockPortfolios = [
  { id: 1, uuid: 'p1', name: 'Mein Portfolio', referenceAccountName: null, isRetired: false, transactionsCount: 10, holdingsCount: 5 },
];

const mockAccounts = [
  { id: 1, uuid: 'a1', name: 'Girokonto', currency: 'EUR', isRetired: false, transactionsCount: 50, balance: 10000 },
];

const mockPreview = {
  bank: 'Scalable Capital',
  transactions: [
    {
      date: '2024-01-15',
      txnType: 'Buy',
      securityName: 'MSCI World ETF',
      isin: 'IE00BK5BQT80',
      wkn: 'A2PKXG',
      shares: 1.5,
      pricePerShare: 100,
      grossAmount: 150,
      fees: 0.99,
      taxes: 0,
      netAmount: 150.99,
      currency: 'EUR',
      note: null,
      exchangeRate: null,
      forexCurrency: null,
    },
  ],
  warnings: [],
  newSecurities: [],
  matchedSecurities: [],
  potentialDuplicates: [],
};

const mockImportResult = {
  success: true,
  bank: 'Scalable Capital',
  transactionsImported: 1,
  transactionsSkipped: 0,
  securitiesCreated: 0,
  errors: [],
  warnings: [],
};

describe('PdfImportModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Setup default mock returns
    (getSupportedBanks as ReturnType<typeof vi.fn>).mockResolvedValue(mockBanks);
    (getPortfolios as ReturnType<typeof vi.fn>).mockResolvedValue(mockPortfolios);
    (getAccounts as ReturnType<typeof vi.fn>).mockResolvedValue(mockAccounts);
    (previewPdfImport as ReturnType<typeof vi.fn>).mockResolvedValue(mockPreview);
    (importPdfTransactions as ReturnType<typeof vi.fn>).mockResolvedValue(mockImportResult);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Initial Render', () => {
    it('should render when isOpen is true', async () => {
      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF Import')).toBeInTheDocument();
      });
    });

    it('should not render when isOpen is false', () => {
      render(<PdfImportModal isOpen={false} onClose={() => {}} />);

      expect(screen.queryByText('PDF Import')).not.toBeInTheDocument();
    });

    it('should load supported banks on open', async () => {
      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(getSupportedBanks).toHaveBeenCalledTimes(1);
        expect(screen.getByText('DKB')).toBeInTheDocument();
        expect(screen.getByText('ING')).toBeInTheDocument();
      });
    });

    it('should load portfolios and accounts on open', async () => {
      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(getPortfolios).toHaveBeenCalledTimes(1);
        expect(getAccounts).toHaveBeenCalledTimes(1);
      });
    });

    it('should show file selection prompt', async () => {
      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });
    });
  });

  describe('Error Handling - Initial Load', () => {
    it('should handle getSupportedBanks error gracefully', async () => {
      (getSupportedBanks as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Network error'));

      // Should not throw
      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF Import')).toBeInTheDocument();
      });
    });

    it('should handle getPortfolios error gracefully', async () => {
      (getPortfolios as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Database error'));

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF Import')).toBeInTheDocument();
      });
    });

    it('should handle getAccounts error gracefully', async () => {
      (getAccounts as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Database error'));

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF Import')).toBeInTheDocument();
      });
    });
  });

  describe('File Selection', () => {
    it('should open file dialog when clicking upload area', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(open).toHaveBeenCalledWith({
          multiple: false,
          filters: [{ name: 'PDF', extensions: ['pdf'] }],
        });
      });
    });

    it('should handle file selection cancellation', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue(null);

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        // Should stay on select step
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
        expect(previewPdfImport).not.toHaveBeenCalled();
      });
    });

    it('should call previewPdfImport after file selection', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(previewPdfImport).toHaveBeenCalledWith('/path/to/test.pdf');
      });
    });
  });

  describe('Error Handling - File Selection', () => {
    it('should display error when file dialog fails', async () => {
      (open as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Dialog error'));

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText('Dialog error')).toBeInTheDocument();
      });
    });

    it('should display error when previewPdfImport fails', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (previewPdfImport as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('PDF parsing failed'));

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText('PDF parsing failed')).toBeInTheDocument();
      });
    });

    it('should display error for unsupported bank', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (previewPdfImport as ReturnType<typeof vi.fn>).mockRejectedValue(
        new Error('Could not detect bank from PDF content')
      );

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText(/Could not detect bank/)).toBeInTheDocument();
      });
    });
  });

  describe('Preview Step', () => {
    it('should show preview after successful file selection', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText('Scalable Capital')).toBeInTheDocument();
        expect(screen.getByText('Erkannte Transaktionen')).toBeInTheDocument();
      });
    });

    it('should display transaction details in preview', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText('MSCI World ETF')).toBeInTheDocument();
        expect(screen.getByText('IE00BK5BQT80')).toBeInTheDocument();
        // Transaction type is now a select element with value "Buy" and label "Kauf"
        const typeSelects = screen.getAllByRole('combobox');
        // First select is "Alle ändern", second is the transaction type dropdown
        const txnTypeSelect = typeSelects[1];
        expect(txnTypeSelect).toHaveValue('Buy');
      });
    });

    it('should display warnings if present', async () => {
      const previewWithWarnings = {
        ...mockPreview,
        warnings: ['[Warnung] betrag: Betrag konnte nicht geparst werden (Wert: "abc")'],
      };
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (previewPdfImport as ReturnType<typeof vi.fn>).mockResolvedValue(previewWithWarnings);

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        // Both the summary card and warning section have "Warnungen" text
        const warnungsElements = screen.getAllByText('Warnungen');
        expect(warnungsElements.length).toBeGreaterThanOrEqual(2);
      });
    });

    it('should display potential duplicates if present', async () => {
      const previewWithDuplicates = {
        ...mockPreview,
        potentialDuplicates: [
          {
            transactionIndex: 0,
            existingTxnId: 123,
            date: '2024-01-15',
            amount: 150.99,
            securityName: 'MSCI World ETF',
            txnType: 'Buy',
          },
        ],
      };
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (previewPdfImport as ReturnType<typeof vi.fn>).mockResolvedValue(previewWithDuplicates);

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText(/Mögliche Duplikate/)).toBeInTheDocument();
      });
    });

    it('should show new securities section if present', async () => {
      const previewWithNewSecurities = {
        ...mockPreview,
        newSecurities: [
          { isin: 'IE00BK5BQT80', wkn: 'A2PKXG', name: 'MSCI World ETF' },
        ],
      };
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (previewPdfImport as ReturnType<typeof vi.fn>).mockResolvedValue(previewWithNewSecurities);

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText('Neue Wertpapiere (werden angelegt)')).toBeInTheDocument();
      });
    });
  });

  describe('Configure Step', () => {
    it('should navigate to configure step when clicking continue', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      await waitFor(() => {
        expect(screen.getByText('Import-Einstellungen')).toBeInTheDocument();
      });
    });

    it('should show portfolio and account selectors', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      await waitFor(() => {
        expect(screen.getByText('Portfolio')).toBeInTheDocument();
        expect(screen.getByText('Verrechnungskonto')).toBeInTheDocument();
      });
    });

    it('should pre-select first portfolio and account', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      await waitFor(() => {
        // Check that import button is enabled (means selectors are pre-filled)
        const importButton = screen.getByText('Import starten');
        expect(importButton).not.toBeDisabled();
      });
    });
  });

  describe('Import Execution', () => {
    it('should call importPdfTransactions when clicking import', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      // Go to configure
      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      // Start import
      await waitFor(() => {
        expect(screen.getByText('Import starten')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Import starten'));

      await waitFor(() => {
        expect(importPdfTransactions).toHaveBeenCalledWith(
          '/path/to/test.pdf',
          1, // portfolio id
          1, // account id
          true, // createMissingSecurities
          true, // skipDuplicates
          undefined, // typeOverrides (none changed)
          undefined // feeOverrides (none changed)
        );
      });
    });

    it('should show success message after import', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      // Go to configure
      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      // Start import
      await waitFor(() => {
        expect(screen.getByText('Import starten')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Import starten'));

      await waitFor(() => {
        expect(screen.getByText('Import erfolgreich!')).toBeInTheDocument();
        expect(screen.getByText(/1 Transaktionen wurden importiert/)).toBeInTheDocument();
      });
    });

    it('should call onSuccess callback after successful import', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      const onSuccess = vi.fn();

      render(<PdfImportModal isOpen={true} onClose={() => {}} onSuccess={onSuccess} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      // Go to configure
      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      // Start import
      await waitFor(() => {
        expect(screen.getByText('Import starten')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Import starten'));

      await waitFor(() => {
        expect(onSuccess).toHaveBeenCalledTimes(1);
      });
    });
  });

  describe('Error Handling - Import', () => {
    it('should display error when import fails', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (importPdfTransactions as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Import failed'));

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      // Go to configure
      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      // Start import
      await waitFor(() => {
        expect(screen.getByText('Import starten')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Import starten'));

      await waitFor(() => {
        expect(screen.getByText('Import failed')).toBeInTheDocument();
      });
    });

    it('should return to configure step on import error', async () => {
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (importPdfTransactions as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Import failed'));

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      // Go to configure
      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      // Start import
      await waitFor(() => {
        expect(screen.getByText('Import starten')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Import starten'));

      await waitFor(() => {
        // Should be back on configure step with error shown
        expect(screen.getByText('Import-Einstellungen')).toBeInTheDocument();
        expect(screen.getByText('Import failed')).toBeInTheDocument();
      });
    });

    it('should show import errors in result', async () => {
      const resultWithErrors = {
        ...mockImportResult,
        success: false,
        errors: ['Transaction 1: Missing security', 'Transaction 2: Invalid date'],
      };
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (importPdfTransactions as ReturnType<typeof vi.fn>).mockResolvedValue(resultWithErrors);

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      // Go to configure
      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      // Start import
      await waitFor(() => {
        expect(screen.getByText('Import starten')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Import starten'));

      await waitFor(() => {
        expect(screen.getByText('Import fehlgeschlagen')).toBeInTheDocument();
        expect(screen.getByText('Transaction 1: Missing security')).toBeInTheDocument();
        expect(screen.getByText('Transaction 2: Invalid date')).toBeInTheDocument();
      });
    });
  });

  describe('Modal Close', () => {
    it('should call onClose when clicking close button', async () => {
      const onClose = vi.fn();
      render(<PdfImportModal isOpen={true} onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText('PDF Import')).toBeInTheDocument();
      });

      // Find and click the X button
      const closeButton = screen.getByRole('button', { name: '' }); // X icon button
      fireEvent.click(closeButton);

      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it('should reset state when modal closes', async () => {
      const { rerender } = render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Wait for initial render
      await waitFor(() => {
        expect(screen.getByText('PDF Import')).toBeInTheDocument();
      });

      // Close modal
      rerender(<PdfImportModal isOpen={false} onClose={() => {}} />);

      // Reopen modal
      rerender(<PdfImportModal isOpen={true} onClose={() => {}} />);

      await waitFor(() => {
        // Should be back on select step
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });
    });
  });

  describe('Edge Cases', () => {
    it('should handle empty portfolios list', async () => {
      (getPortfolios as ReturnType<typeof vi.fn>).mockResolvedValue([]);
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      // Go to configure
      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      await waitFor(() => {
        // Import button should be disabled
        const importButton = screen.getByText('Import starten');
        expect(importButton).toBeDisabled();
      });
    });

    it('should handle empty accounts list', async () => {
      (getAccounts as ReturnType<typeof vi.fn>).mockResolvedValue([]);
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      // Go to configure
      await waitFor(() => {
        expect(screen.getByText('Weiter zur Konfiguration')).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText('Weiter zur Konfiguration'));

      await waitFor(() => {
        // Import button should be disabled
        const importButton = screen.getByText('Import starten');
        expect(importButton).toBeDisabled();
      });
    });

    it('should handle preview with zero transactions', async () => {
      const emptyPreview = {
        ...mockPreview,
        transactions: [],
      };
      (open as ReturnType<typeof vi.fn>).mockResolvedValue('/path/to/test.pdf');
      (previewPdfImport as ReturnType<typeof vi.fn>).mockResolvedValue(emptyPreview);

      render(<PdfImportModal isOpen={true} onClose={() => {}} />);

      // Select file
      await waitFor(() => {
        expect(screen.getByText('PDF-Datei auswählen')).toBeInTheDocument();
      });

      const uploadArea = screen.getByText('PDF-Datei auswählen').parentElement!;
      fireEvent.click(uploadArea);

      await waitFor(() => {
        // Preview step should still show even with 0 transactions
        expect(screen.getByText('Scalable Capital')).toBeInTheDocument();
        expect(screen.getByText('Erkannte Transaktionen')).toBeInTheDocument();
        // Table should be empty (only header)
        expect(screen.queryByText('MSCI World ETF')).not.toBeInTheDocument();
      });
    });
  });
});
