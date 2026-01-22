/**
 * Extracted Transactions Preview - Shows transactions extracted from images.
 *
 * SECURITY: Transactions are displayed as suggestions only.
 * User must explicitly confirm import - no auto-execution.
 */

import { useState, useEffect } from 'react';
import { Receipt, Check, X, AlertTriangle, ChevronDown, ChevronUp, ArrowRight, Briefcase, Info } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { cn } from '../../lib/utils';

export interface Portfolio {
  id: number;
  name: string;
}

export interface ExtractedTransaction {
  date: string;
  txnType: string;
  securityName?: string;
  isin?: string;
  wkn?: string;
  ticker?: string;
  shares?: number;
  // Primary amount (in account/portfolio currency after conversion)
  amount?: number;
  currency: string;
  // Original foreign currency details (if different from account currency)
  grossAmount?: number;
  grossCurrency?: string;
  exchangeRate?: number;
  // Price per share
  pricePerShare?: number;
  pricePerShareCurrency?: string;
  // Fees and taxes (can also have foreign currency equivalents)
  fees?: number;
  feesForeign?: number;
  feesForeignCurrency?: string;
  taxes?: number;
  taxesForeign?: number;
  taxesForeignCurrency?: string;
  // Additional info
  note?: string;
  valueDate?: string;
  orderId?: string;
}

export interface ExtractedTransactionsPayload {
  transactions: ExtractedTransaction[];
  sourceDescription?: string;
}

// Response from enrich_extracted_transactions command
interface EnrichedTransactionResponse {
  date: string;
  txn_type: string;
  security_name: string | null;
  isin: string | null;
  shares: number | null;
  shares_from_holdings: boolean;
  gross_amount: number | null;
  gross_currency: string | null;
  amount: number | null;
  currency: string;
  fees: number | null;
  fees_foreign: number | null;
  fees_foreign_currency: string | null;
  exchange_rate: number | null;
  taxes: number | null;
  note: string | null;
}

interface ExtractedTransactionsPreviewProps {
  payload: ExtractedTransactionsPayload;
  portfolios: Portfolio[];
  defaultPortfolioId?: number;
  onConfirm: (transactions: ExtractedTransaction[], portfolioId: number | null) => void;
  onDiscard: () => void;
  isImporting?: boolean;
  className?: string;
}

function formatCurrency(value: number | undefined, currency: string): string {
  if (value === undefined || value === null) return '-';
  return new Intl.NumberFormat('de-DE', {
    style: 'currency',
    currency: currency,
  }).format(value);
}

function formatNumber(value: number | undefined, decimals: number = 2): string {
  if (value === undefined || value === null) return '-';
  return value.toLocaleString('de-DE', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

function parseMonthName(value: string): number | null {
  const month = value.toLowerCase();
  switch (month) {
    case 'jan':
    case 'januar':
    case 'january':
      return 1;
    case 'feb':
    case 'februar':
    case 'february':
      return 2;
    case 'mar':
    case 'mär':
    case 'maerz':
    case 'märz':
    case 'march':
      return 3;
    case 'apr':
    case 'april':
      return 4;
    case 'may':
    case 'mai':
      return 5;
    case 'jun':
    case 'juni':
    case 'june':
      return 6;
    case 'jul':
    case 'juli':
    case 'july':
      return 7;
    case 'aug':
    case 'august':
      return 8;
    case 'sep':
    case 'sept':
    case 'september':
      return 9;
    case 'oct':
    case 'okt':
    case 'oktober':
    case 'october':
      return 10;
    case 'nov':
    case 'november':
      return 11;
    case 'dec':
    case 'dez':
    case 'dezember':
    case 'december':
      return 12;
    default:
      return null;
  }
}

function parseNumericDate(value: string, sep: string, preferMdy: boolean): Date | null {
  const parts = value.split(sep).map((p) => p.trim());
  if (parts.length !== 3) return null;
  const [p0, p1, p2] = parts;
  if (p0.length === 4) {
    const year = Number(p0);
    const month = Number(p1);
    const day = Number(p2);
    if (!Number.isFinite(year) || !Number.isFinite(month) || !Number.isFinite(day)) return null;
    return new Date(year, month - 1, day);
  }
  if (p2.length === 4) {
    const year = Number(p2);
    const a = Number(p0);
    const b = Number(p1);
    if (!Number.isFinite(year) || !Number.isFinite(a) || !Number.isFinite(b)) return null;
    let month = b;
    let day = a;
    if (a > 12 && b <= 12) {
      month = b;
      day = a;
    } else if (b > 12 && a <= 12) {
      month = a;
      day = b;
    } else if (preferMdy) {
      month = a;
      day = b;
    } else {
      month = b;
      day = a;
    }
    return new Date(year, month - 1, day);
  }
  return null;
}

function parseMonthNameDate(value: string): Date | null {
  const cleaned = value.toLowerCase().replace(/[,\.]/g, ' ');
  const tokens = cleaned.split(/\s+/).filter(Boolean);
  if (tokens.length < 3) return null;

  const dayFirst = Number(tokens[0]);
  const monthMiddle = parseMonthName(tokens[1]);
  const yearLast = Number(tokens[2]);
  if (Number.isFinite(dayFirst) && monthMiddle && Number.isFinite(yearLast)) {
    return new Date(yearLast, monthMiddle - 1, dayFirst);
  }

  const monthFirst = parseMonthName(tokens[0]);
  const dayMiddle = Number(tokens[1]);
  const yearEnd = Number(tokens[2]);
  if (monthFirst && Number.isFinite(dayMiddle) && Number.isFinite(yearEnd)) {
    return new Date(yearEnd, monthFirst - 1, dayMiddle);
  }

  return null;
}

function formatDate(dateStr: string, currency?: string): string {
  const trimmed = dateStr.trim();
  if (!trimmed) return dateStr;

  const preferMdy = currency === 'USD';
  const isoMatch = trimmed.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (isoMatch) {
    const year = Number(isoMatch[1]);
    const month = Number(isoMatch[2]);
    const day = Number(isoMatch[3]);
    return new Date(year, month - 1, day).toLocaleDateString('de-DE');
  }

  const date =
    parseNumericDate(trimmed, '.', preferMdy) ||
    parseNumericDate(trimmed, '/', preferMdy) ||
    parseNumericDate(trimmed, '-', preferMdy) ||
    parseMonthNameDate(trimmed);

  if (date && !Number.isNaN(date.getTime())) {
    return date.toLocaleDateString('de-DE');
  }

  return dateStr;
}

/**
 * Check if a date might be problematic (misread by AI)
 * Returns a warning message if suspicious, null otherwise
 */
function getDateWarning(dateStr: string): string | null {
  const trimmed = dateStr.trim();
  if (!trimmed) return null;

  const isoMatch = trimmed.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (!isoMatch) return null;

  const year = Number(isoMatch[1]);
  const month = Number(isoMatch[2]);
  const day = Number(isoMatch[3]);
  const date = new Date(year, month - 1, day);

  const now = new Date();
  const oneMonthFromNow = new Date(now.getTime() + 30 * 24 * 60 * 60 * 1000);
  const fiveYearsAgo = new Date(now.getTime() - 5 * 365 * 24 * 60 * 60 * 1000);

  // Date in the future (more than 30 days)
  if (date > oneMonthFromNow) {
    return 'Datum liegt in der Zukunft - bitte überprüfen';
  }

  // Date more than 5 years ago
  if (date < fiveYearsAgo) {
    return 'Datum liegt weit in der Vergangenheit - bitte überprüfen';
  }

  // Check for potential month/day swap (if both values are <= 12)
  if (month <= 12 && day <= 12 && month !== day) {
    // Could be MM/DD vs DD/MM ambiguity
    const swappedDate = new Date(year, day - 1, month);
    const daysDiff = Math.abs(date.getTime() - swappedDate.getTime()) / (24 * 60 * 60 * 1000);

    // If swapped date is significantly different and closer to today, warn
    if (daysDiff > 14) {
      const distToNow = Math.abs(date.getTime() - now.getTime());
      const swappedDistToNow = Math.abs(swappedDate.getTime() - now.getTime());

      if (swappedDistToNow < distToNow * 0.5) {
        return `Datum könnte auch ${swappedDate.toLocaleDateString('de-DE')} sein - bitte überprüfen`;
      }
    }
  }

  return null;
}

function normalizeTxnType(txnType: string): string {
  const normalized = txnType.trim().toUpperCase().replace(/[\s-]+/g, '_');
  switch (normalized) {
    case 'DIVIDEND':
    case 'DIVIDENDS':
    case 'DIVIDENDE':
    case 'DIVIDENDEN':
    case 'AUSSCHÜTTUNG':
    case 'AUSSCHUETTUNG':
    case 'ERTRAG':
    case 'ERTRAGSGUTSCHRIFT':
    case 'DIVIDENDENGUTSCHRIFT':
      return 'DIVIDENDS';
    case 'KAUF':
      return 'BUY';
    case 'VERKAUF':
      return 'SELL';
    case 'EINLIEFERUNG':
      return 'DELIVERY_INBOUND';
    case 'AUSLIEFERUNG':
      return 'DELIVERY_OUTBOUND';
    case 'UMBUCHUNG_EIN':
    case 'UMBUCHUNG_EINGANG':
      return 'TRANSFER_IN';
    case 'UMBUCHUNG_AUS':
    case 'UMBUCHUNG_AUSGANG':
      return 'TRANSFER_OUT';
    case 'EINZAHLUNG':
    case 'EINLAGE':
      return 'DEPOSIT';
    case 'AUSZAHLUNG':
    case 'ENTNAHME':
      return 'REMOVAL';
    case 'ZINS':
    case 'ZINSEN':
      return 'INTEREST';
    case 'GEBUEHREN':
    case 'GEBÜHREN':
      return 'FEES';
    case 'STEUERN':
      return 'TAXES';
    default:
      return normalized;
  }
}

function getTxnTypeLabel(txnType: string): string {
  const normalized = normalizeTxnType(txnType);
  const labels: Record<string, string> = {
    BUY: 'Kauf',
    SELL: 'Verkauf',
    DIVIDENDS: 'Dividende',
    DEPOSIT: 'Einzahlung',
    REMOVAL: 'Auszahlung',
    INTEREST: 'Zinsen',
    FEES: 'Gebühren',
    TRANSFER_IN: 'Umbuchung Ein',
    TRANSFER_OUT: 'Umbuchung Aus',
    DELIVERY_INBOUND: 'Einlieferung',
    DELIVERY_OUTBOUND: 'Auslieferung',
  };
  return labels[normalized] || txnType;
}

function getTxnTypeColor(txnType: string): string {
  switch (normalizeTxnType(txnType)) {
    case 'BUY':
    case 'DEPOSIT':
    case 'TRANSFER_IN':
    case 'DIVIDENDS':
    case 'INTEREST':
    case 'DELIVERY_INBOUND':
      return 'text-green-600';
    case 'SELL':
    case 'REMOVAL':
    case 'TRANSFER_OUT':
    case 'FEES':
    case 'DELIVERY_OUTBOUND':
      return 'text-red-600';
    default:
      return 'text-foreground';
  }
}

function isDividendTxnType(txnType: string): boolean {
  return normalizeTxnType(txnType) === 'DIVIDENDS';
}

function hasValidShares(shares: number | undefined): boolean {
  return typeof shares === 'number' && shares > 0;
}

function hasForeignCurrency(txn: ExtractedTransaction): boolean {
  return !!(txn.grossCurrency && txn.grossCurrency !== txn.currency);
}

function getTotalFees(txn: ExtractedTransaction): number | undefined {
  let total = 0;
  let hasFees = false;
  if (typeof txn.fees === 'number') {
    total += txn.fees;
    hasFees = true;
  }
  if (
    typeof txn.feesForeign === 'number' &&
    txn.feesForeignCurrency &&
    txn.feesForeignCurrency === txn.currency
  ) {
    total += txn.feesForeign;
    hasFees = true;
  }
  return hasFees ? total : undefined;
}

export function ExtractedTransactionsPreview({
  payload,
  portfolios,
  defaultPortfolioId,
  onConfirm,
  onDiscard,
  isImporting = false,
  className,
}: ExtractedTransactionsPreviewProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [selectedPortfolioId, setSelectedPortfolioId] = useState<number | null>(
    defaultPortfolioId ?? (portfolios.length > 0 ? portfolios[0].id : null)
  );
  const { transactions: originalTransactions, sourceDescription } = payload;

  // Enriched transactions with shares from holdings for dividends
  const [enrichedTransactions, setEnrichedTransactions] = useState<ExtractedTransaction[]>(originalTransactions);
  const [sharesFromHoldingsMap, setSharesFromHoldingsMap] = useState<Map<number, boolean>>(new Map());
  const [isEnriching, setIsEnriching] = useState(false);

  // Update selected portfolio when default changes or portfolios load
  useEffect(() => {
    if (selectedPortfolioId === null && portfolios.length > 0) {
      setSelectedPortfolioId(defaultPortfolioId ?? portfolios[0].id);
    }
  }, [portfolios, defaultPortfolioId, selectedPortfolioId]);

  // Enrich transactions with holdings data (for dividends without shares)
  useEffect(() => {
    const enrichTransactions = async () => {
      // Check if any transaction is a dividend without shares
      const hasDividendWithoutShares = originalTransactions.some(
        (txn) => isDividendTxnType(txn.txnType) && !hasValidShares(txn.shares)
      );

      if (!hasDividendWithoutShares) {
        setEnrichedTransactions(originalTransactions);
        return;
      }

      setIsEnriching(true);
      try {
        // Call backend to enrich transactions with holdings data
        const enriched = await invoke<EnrichedTransactionResponse[]>('enrich_extracted_transactions', {
          transactions: originalTransactions.map((txn) => ({
            date: txn.date,
            txn_type: txn.txnType,
            security_name: txn.securityName || null,
            isin: txn.isin || null,
            shares: txn.shares ?? null,
            gross_amount: txn.grossAmount ?? null,
            gross_currency: txn.grossCurrency || null,
            amount: txn.amount ?? null,
            currency: txn.currency,
            fees: txn.fees ?? null,
            fees_foreign: txn.feesForeign ?? null,
            fees_foreign_currency: txn.feesForeignCurrency || null,
            exchange_rate: txn.exchangeRate ?? null,
            taxes: txn.taxes ?? null,
            note: txn.note || null,
          })),
        });

        // Map enriched data back to ExtractedTransaction format
        const newMap = new Map<number, boolean>();
        const mergedTransactions = originalTransactions.map((txn, index) => {
          const enrichedTxn = enriched[index];
          let nextTxn = txn;
          if (enrichedTxn && enrichedTxn.shares_from_holdings && enrichedTxn.shares !== null) {
            newMap.set(index, true);
            nextTxn = {
              ...txn,
              shares: enrichedTxn.shares,
            };
          }
          if (isDividendTxnType(nextTxn.txnType) && hasValidShares(nextTxn.shares)) {
            const hasPrice = typeof nextTxn.pricePerShare === 'number' && nextTxn.pricePerShare > 0;
            if (!hasPrice) {
              const grossCurrency = nextTxn.grossCurrency || nextTxn.currency;
              const grossAmount =
                typeof nextTxn.grossAmount === 'number' && nextTxn.grossAmount > 0
                  ? nextTxn.grossAmount
                  : typeof nextTxn.amount === 'number' && typeof nextTxn.taxes === 'number'
                    ? nextTxn.amount + nextTxn.taxes
                    : undefined;
              if (grossAmount && grossAmount > 0) {
                nextTxn = {
                  ...nextTxn,
                  pricePerShare: grossAmount / (nextTxn.shares as number),
                  pricePerShareCurrency: grossCurrency,
                };
              }
            }
          }
          return nextTxn;
        });

        setEnrichedTransactions(mergedTransactions);
        setSharesFromHoldingsMap(newMap);
      } catch (err) {
        console.error('Failed to enrich transactions:', err);
        // Fall back to original transactions on error
        setEnrichedTransactions(originalTransactions);
      } finally {
        setIsEnriching(false);
      }
    };

    enrichTransactions();
  }, [originalTransactions]);

  // Use enriched transactions for display
  const transactions = enrichedTransactions;

  if (transactions.length === 0) {
    return null;
  }

  // Check if any transaction has foreign currency
  const hasFxTransactions = transactions.some(hasForeignCurrency);

  // Check if any transaction requires a portfolio (BUY, SELL, DELIVERY, etc.)
  const needsPortfolio = transactions.some((txn) =>
    ['BUY', 'SELL', 'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND', 'TRANSFER_IN', 'TRANSFER_OUT'].includes(txn.txnType)
  );

  return (
    <div
      className={cn(
        'rounded-lg border border-amber-500/50 bg-amber-500/5 overflow-hidden',
        className
      )}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between p-3 bg-amber-500/10 cursor-pointer"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-2">
          <Receipt className="h-4 w-4 text-amber-600" />
          <span className="font-medium text-sm">
            {transactions.length === 1
              ? '1 Transaktion erkannt'
              : `${transactions.length} Transaktionen erkannt`}
            {hasFxTransactions && (
              <span className="text-amber-600 ml-1">(mit Währungsumrechnung)</span>
            )}
          </span>
        </div>
        <button
          type="button"
          className="p-1 hover:bg-amber-500/20 rounded"
          onClick={(e) => {
            e.stopPropagation();
            setIsExpanded(!isExpanded);
          }}
        >
          {isExpanded ? (
            <ChevronUp className="h-4 w-4" />
          ) : (
            <ChevronDown className="h-4 w-4" />
          )}
        </button>
      </div>

      {isExpanded && (
        <>
          {/* Source description */}
          {sourceDescription && (
            <div className="px-3 py-2 text-xs text-muted-foreground border-b border-amber-500/20">
              Quelle: {sourceDescription}
            </div>
          )}

          {/* Portfolio selection (only shown if transactions need a portfolio) */}
          {needsPortfolio && portfolios.length > 0 && (
            <div className="px-3 py-2 border-b border-amber-500/20">
              <div className="flex items-center gap-2">
                <Briefcase className="h-4 w-4 text-muted-foreground" />
                <label htmlFor="portfolio-select" className="text-sm text-muted-foreground">
                  Importieren in:
                </label>
                <select
                  id="portfolio-select"
                  value={selectedPortfolioId ?? ''}
                  onChange={(e) => setSelectedPortfolioId(Number(e.target.value))}
                  className="flex-1 text-sm bg-background border border-border rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-primary"
                >
                  {portfolios.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          )}

          {/* Transactions list */}
          <div className="divide-y divide-amber-500/10">
            {transactions.map((txn, index) => (
              <div key={index} className="p-3 hover:bg-muted/20">
                {/* Main row */}
                <div className="flex items-start justify-between gap-3">
                  {/* Left: Date, Type, Security */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-sm font-medium">{formatDate(txn.date, txn.currency)}</span>
                      <span className={cn('text-sm font-semibold', getTxnTypeColor(txn.txnType))}>
                        {getTxnTypeLabel(txn.txnType)}
                      </span>
                    </div>
                    {/* Date warning if potentially misread */}
                    {getDateWarning(txn.date) && (
                      <div className="flex items-center gap-1 mt-1 text-xs text-amber-600">
                        <AlertTriangle className="h-3 w-3" />
                        <span>{getDateWarning(txn.date)}</span>
                      </div>
                    )}
                    {txn.securityName && (
                      <div className="mt-1">
                        <span className="text-sm font-medium">{txn.securityName}</span>
                        {(txn.isin || txn.ticker) && (
                          <span className="text-xs text-muted-foreground ml-2">
                            {txn.ticker && <span>{txn.ticker}</span>}
                            {txn.ticker && txn.isin && <span> · </span>}
                            {txn.isin && <span>{txn.isin}</span>}
                          </span>
                        )}
                      </div>
                    )}
                  </div>

                  {/* Right: Shares & Amount */}
                  <div className="text-right shrink-0">
                    {hasValidShares(txn.shares) && (
                      <div className="text-sm">
                        {formatNumber(txn.shares, 4)} Stk.
                        {sharesFromHoldingsMap.get(index) && (
                          <span
                            className="inline-flex items-center ml-1 text-xs text-blue-600"
                            title="Stückzahl aus aktuellem Bestand ermittelt"
                          >
                            <Info className="h-3 w-3" />
                          </span>
                        )}
                        {txn.pricePerShare !== undefined && (
                          <span className="text-muted-foreground ml-1">
                            @ {formatNumber(txn.pricePerShare, 2)} {txn.pricePerShareCurrency || txn.grossCurrency || txn.currency}
                          </span>
                        )}
                      </div>
                    )}
                    <div className="text-sm font-medium">
                      {hasForeignCurrency(txn) ? (
                        <div className="flex items-center justify-end gap-1">
                          <span className="text-muted-foreground">
                            {formatCurrency(txn.grossAmount, txn.grossCurrency!)}
                          </span>
                          <ArrowRight className="h-3 w-3 text-muted-foreground" />
                          <span>{formatCurrency(txn.amount, txn.currency)}</span>
                        </div>
                      ) : (
                        formatCurrency(txn.amount, txn.currency)
                      )}
                    </div>
                  </div>
                </div>

                {/* Details row */}
                <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                  {/* Exchange rate */}
                  {hasForeignCurrency(txn) && txn.exchangeRate && (
                    <span>
                      Kurs: 1 {txn.grossCurrency} = {formatNumber(txn.exchangeRate, 4)} {txn.currency}
                    </span>
                  )}

                  {/* Fees */}
                  {(typeof txn.fees === 'number' || typeof txn.feesForeign === 'number') && (
                    <span>
                      Gebühren:
                      {txn.feesForeign && txn.feesForeignCurrency && txn.feesForeignCurrency !== txn.currency ? (
                        <>
                          {' '}{formatCurrency(txn.feesForeign, txn.feesForeignCurrency)}
                          {txn.fees && (
                            <> → {formatCurrency(txn.fees, txn.currency)}</>
                          )}
                        </>
                      ) : (
                        <> {formatCurrency(getTotalFees(txn), txn.currency)}</>
                      )}
                    </span>
                  )}

                  {/* Taxes */}
                  {(txn.taxes || txn.taxesForeign) && (
                    <span>
                      Steuern:
                      {txn.taxesForeign && txn.taxesForeignCurrency && txn.taxesForeignCurrency !== txn.currency ? (
                        <>
                          {' '}{formatCurrency(txn.taxesForeign, txn.taxesForeignCurrency)}
                          {txn.taxes && (
                            <> → {formatCurrency(txn.taxes, txn.currency)}</>
                          )}
                        </>
                      ) : (
                        <> {formatCurrency(txn.taxes, txn.currency)}</>
                      )}
                    </span>
                  )}

                  {/* Value date */}
                  {txn.valueDate && txn.valueDate !== txn.date && (
                    <span>Valuta: {formatDate(txn.valueDate, txn.currency)}</span>
                  )}

                  {/* Order ID */}
                  {txn.orderId && (
                    <span>Ref: {txn.orderId}</span>
                  )}
                </div>

                {/* Note */}
                {txn.note && (
                  <div className="mt-1 text-xs italic text-muted-foreground">
                    {txn.note}
                  </div>
                )}
              </div>
            ))}
          </div>

          {/* Info about shares from holdings */}
          {sharesFromHoldingsMap.size > 0 && (
            <div className="px-3 py-2 border-t border-blue-500/20 bg-blue-500/5">
              <div className="flex items-start gap-2 text-xs text-blue-600">
                <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
                <span>
                  Bei {sharesFromHoldingsMap.size === 1 ? 'einer Dividende' : `${sharesFromHoldingsMap.size} Dividenden`} wurde
                  die Stückzahl aus dem aktuellen Bestand ermittelt.
                </span>
              </div>
            </div>
          )}

          {/* Loading indicator while enriching */}
          {isEnriching && (
            <div className="px-3 py-2 border-t border-amber-500/20 bg-muted/20">
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <div className="h-3 w-3 border-2 border-amber-500/30 border-t-amber-500 rounded-full animate-spin" />
                <span>Ermittle Stückzahlen aus Bestand...</span>
              </div>
            </div>
          )}

          {/* Warning */}
          <div className="px-3 py-2 border-t border-amber-500/20 bg-muted/20">
            <div className="flex items-start gap-2 text-xs text-muted-foreground">
              <AlertTriangle className="h-3.5 w-3.5 text-amber-500 mt-0.5 shrink-0" />
              <span>
                Bitte prüfe die extrahierten Daten vor dem Import.
                Die KI-Erkennung kann Fehler enthalten.
              </span>
            </div>
          </div>

          {/* Actions */}
          <div className="flex gap-2 p-3 border-t border-amber-500/20">
            <button
              type="button"
              onClick={onDiscard}
              disabled={isImporting}
              className={cn(
                'flex-1 flex items-center justify-center gap-2 px-4 py-2',
                'text-sm font-medium rounded-lg',
                'border border-border bg-muted hover:bg-muted/80',
                'disabled:opacity-50 disabled:cursor-not-allowed',
                'transition-colors'
              )}
            >
              <X className="h-4 w-4" />
              Verwerfen
            </button>
            <button
              type="button"
              onClick={() => onConfirm(transactions, selectedPortfolioId)}
              disabled={isImporting || isEnriching}
              className={cn(
                'flex-1 flex items-center justify-center gap-2 px-4 py-2',
                'text-sm font-medium rounded-lg',
                'bg-green-600 text-white hover:bg-green-700',
                'disabled:opacity-50 disabled:cursor-not-allowed',
                'transition-colors'
              )}
            >
              {isImporting ? (
                <>
                  <div className="h-4 w-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Importiere...
                </>
              ) : isEnriching ? (
                <>
                  <div className="h-4 w-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Lade...
                </>
              ) : (
                <>
                  <Check className="h-4 w-4" />
                  Importieren
                </>
              )}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

export default ExtractedTransactionsPreview;
