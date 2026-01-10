/**
 * Modal for creating and editing securities.
 */

import { useState, useEffect, useMemo } from 'react';
import { X, HelpCircle, ChevronDown, ChevronUp, ChevronRight, Plus, Trash2 } from 'lucide-react';
import type { SecurityData, CreateSecurityRequest, UpdateSecurityRequest } from '../../lib/types';
import { createSecurity, updateSecurity } from '../../lib/api';
import { useSettingsStore } from '../../store';

// Key-Value Entry Component for attributes/properties editing
interface KeyValueEntry {
  key: string;
  value: string;
}

function KeyValueEditor({
  title,
  entries,
  onChange,
  expanded,
  onToggleExpand,
}: {
  title: string;
  entries: KeyValueEntry[];
  onChange: (entries: KeyValueEntry[]) => void;
  expanded: boolean;
  onToggleExpand: () => void;
}) {
  const addEntry = () => {
    onChange([...entries, { key: '', value: '' }]);
  };

  const updateEntry = (index: number, field: 'key' | 'value', val: string) => {
    const newEntries = [...entries];
    newEntries[index] = { ...newEntries[index], [field]: val };
    onChange(newEntries);
  };

  const removeEntry = (index: number) => {
    onChange(entries.filter((_, i) => i !== index));
  };

  return (
    <div className="border border-border rounded-md overflow-hidden">
      <button
        type="button"
        onClick={onToggleExpand}
        className="w-full flex items-center justify-between p-3 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
      >
        <div className="flex items-center gap-2">
          <ChevronRight
            size={16}
            className={`transition-transform ${expanded ? 'rotate-90' : ''}`}
          />
          <span className="text-sm font-medium">{title}</span>
          <span className="text-xs text-muted-foreground">
            ({entries.filter(e => e.key).length})
          </span>
        </div>
      </button>

      {expanded && (
        <div className="p-3 space-y-2 border-t border-border bg-card">
          {entries.map((entry, index) => (
            <div key={index} className="flex gap-2 items-center">
              <input
                type="text"
                value={entry.key}
                onChange={(e) => updateEntry(index, 'key', e.target.value)}
                placeholder="Schlüssel"
                className="flex-1 px-2 py-1.5 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
              />
              <input
                type="text"
                value={entry.value}
                onChange={(e) => updateEntry(index, 'value', e.target.value)}
                placeholder="Wert"
                className="flex-1 px-2 py-1.5 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
              />
              <button
                type="button"
                onClick={() => removeEntry(index)}
                className="p-1.5 text-muted-foreground hover:text-destructive transition-colors"
              >
                <Trash2 size={14} />
              </button>
            </div>
          ))}
          <button
            type="button"
            onClick={addEntry}
            className="flex items-center gap-1 text-sm text-primary hover:text-primary/80 transition-colors"
          >
            <Plus size={14} />
            Hinzufügen
          </button>
        </div>
      )}
    </div>
  );
}

interface SecurityFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  security?: SecurityData | null; // null = create mode, SecurityData = edit mode
}

const CURRENCIES = ['EUR', 'USD', 'GBP', 'CHF', 'JPY', 'CAD', 'AUD', 'SEK', 'NOK', 'DKK'];

const BASE_FEED_PROVIDERS = [
  { value: '', label: 'Keine' },
  { value: 'YAHOO', label: 'Yahoo Finance' },
  { value: 'ALPHAVANTAGE', label: 'Alpha Vantage' },
  { value: 'COINGECKO', label: 'CoinGecko (Krypto)' },
  { value: 'KRAKEN', label: 'Kraken (Krypto)' },
  { value: 'PORTFOLIO-REPORT', label: 'Portfolio Report' },
  { value: 'MANUAL', label: 'Manuell' },
];

// Yahoo Finance exchange suffixes (like Portfolio Performance)
const YAHOO_EXCHANGES = [
  { value: '', label: 'Automatisch (kein Suffix)', currency: '' },
  { value: '.DE', label: 'XETRA / Frankfurt (.DE)', currency: 'EUR' },
  { value: '.F', label: 'Frankfurt (.F)', currency: 'EUR' },
  { value: '.BE', label: 'Berlin (.BE)', currency: 'EUR' },
  { value: '.DU', label: 'Düsseldorf (.DU)', currency: 'EUR' },
  { value: '.MU', label: 'München (.MU)', currency: 'EUR' },
  { value: '.SG', label: 'Stuttgart (.SG)', currency: 'EUR' },
  { value: '.HM', label: 'Hamburg (.HM)', currency: 'EUR' },
  { value: '.VI', label: 'Wien (.VI)', currency: 'EUR' },
  { value: '.SW', label: 'SIX Swiss (.SW)', currency: 'CHF' },
  { value: '.PA', label: 'Euronext Paris (.PA)', currency: 'EUR' },
  { value: '.AS', label: 'Euronext Amsterdam (.AS)', currency: 'EUR' },
  { value: '.BR', label: 'Euronext Brüssel (.BR)', currency: 'EUR' },
  { value: '.MI', label: 'Mailand (.MI)', currency: 'EUR' },
  { value: '.MC', label: 'Madrid (.MC)', currency: 'EUR' },
  { value: '.L', label: 'London (.L)', currency: 'GBP' },
  { value: '.TO', label: 'Toronto (.TO)', currency: 'CAD' },
  { value: '.AX', label: 'Sydney (.AX)', currency: 'AUD' },
  { value: '.HK', label: 'Hong Kong (.HK)', currency: 'HKD' },
  { value: '.T', label: 'Tokyo (.T)', currency: 'JPY' },
  { value: '.SS', label: 'Shanghai (.SS)', currency: 'CNY' },
  { value: '.SZ', label: 'Shenzhen (.SZ)', currency: 'CNY' },
  { value: '.KS', label: 'Seoul (.KS)', currency: 'KRW' },
];

// Common crypto symbols for CoinGecko
const CRYPTO_SYMBOLS = [
  'BTC', 'ETH', 'BNB', 'XRP', 'ADA', 'SOL', 'DOT', 'DOGE', 'AVAX', 'MATIC',
  'LINK', 'LTC', 'UNI', 'ATOM', 'XLM', 'ALGO', 'AAVE', 'XMR', 'NEAR', 'FTM',
];

export function SecurityFormModal({ isOpen, onClose, onSuccess, security }: SecurityFormModalProps) {
  const isEditMode = !!security;
  const finnhubApiKey = useSettingsStore((state) => state.finnhubApiKey);
  const alphaVantageApiKey = useSettingsStore((state) => state.alphaVantageApiKey);
  const twelveDataApiKey = useSettingsStore((state) => state.twelveDataApiKey);

  // Build provider list based on available API keys
  const feedProviders = useMemo(() => {
    const providers = [...BASE_FEED_PROVIDERS];
    // Insert providers based on available API keys
    const yahooIndex = providers.findIndex((p) => p.value === 'YAHOO');
    if (finnhubApiKey) {
      providers.splice(yahooIndex + 1, 0, { value: 'FINNHUB', label: 'Finnhub' });
    }
    if (twelveDataApiKey) {
      providers.splice(yahooIndex + 1, 0, { value: 'TWELVEDATA', label: 'Twelve Data (CH/EU)' });
    }
    return providers;
  }, [finnhubApiKey, alphaVantageApiKey, twelveDataApiKey]);

  const [formData, setFormData] = useState({
    name: '',
    currency: 'EUR',
    targetCurrency: '',      // Target currency for conversion (PP field)
    isin: '',
    wkn: '',
    ticker: '',
    feed: '',
    feedUrl: '',
    yahooExchange: '', // Exchange suffix for Yahoo (.DE, .L, etc.)
    latestFeed: '',         // Provider for current quotes
    latestFeedUrl: '',      // URL/suffix for current quotes
    latestYahooExchange: '', // Exchange suffix for Yahoo (current quotes)
    note: '',
    isRetired: false,       // Retired flag
  });

  // Attributes and Properties as key-value arrays
  const [attributes, setAttributes] = useState<KeyValueEntry[]>([]);
  const [properties, setProperties] = useState<KeyValueEntry[]>([]);

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showProviderHelp, setShowProviderHelp] = useState(false);
  const [showAttributesExpanded, setShowAttributesExpanded] = useState(false);
  const [showPropertiesExpanded, setShowPropertiesExpanded] = useState(false);

  // Helper to convert Record to KeyValueEntry array
  const recordToEntries = (record: Record<string, string> | undefined): KeyValueEntry[] => {
    if (!record) return [];
    return Object.entries(record).map(([key, value]) => ({ key, value }));
  };

  // Reset form when modal opens or security changes
  useEffect(() => {
    if (isOpen) {
      if (security) {
        // Extract Yahoo exchange suffix from feedUrl if present
        const yahooExchange = security.feed === 'YAHOO' && security.feedUrl?.startsWith('.')
          ? security.feedUrl
          : '';
        const latestYahooExchange = security.latestFeed === 'YAHOO' && security.latestFeedUrl?.startsWith('.')
          ? security.latestFeedUrl
          : '';
        setFormData({
          name: security.name || '',
          currency: security.currency || 'EUR',
          targetCurrency: security.targetCurrency || '',
          isin: security.isin || '',
          wkn: security.wkn || '',
          ticker: security.ticker || '',
          feed: security.feed || '',
          feedUrl: yahooExchange ? '' : (security.feedUrl || ''),
          yahooExchange,
          latestFeed: security.latestFeed || '',
          latestFeedUrl: latestYahooExchange ? '' : (security.latestFeedUrl || ''),
          latestYahooExchange,
          note: security.note || '',
          isRetired: security.isRetired || false,
        });
        // Load attributes and properties
        setAttributes(recordToEntries(security.attributes));
        setProperties(recordToEntries(security.properties));
      } else {
        setFormData({
          name: '',
          currency: 'EUR',
          targetCurrency: '',
          isin: '',
          wkn: '',
          ticker: '',
          feed: '',
          feedUrl: '',
          yahooExchange: '',
          latestFeed: '',
          latestFeedUrl: '',
          latestYahooExchange: '',
          note: '',
          isRetired: false,
        });
        setAttributes([]);
        setProperties([]);
      }
      setError(null);
      setShowAttributesExpanded(false);
      setShowPropertiesExpanded(false);
    }
  }, [isOpen, security]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
  };

  // Helper to convert KeyValueEntry array back to Record
  const entriesToRecord = (entries: KeyValueEntry[]): Record<string, string> | undefined => {
    const filtered = entries.filter(e => e.key.trim());
    if (filtered.length === 0) return undefined;
    return Object.fromEntries(filtered.map(e => [e.key.trim(), e.value]));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsSubmitting(true);

    // For Yahoo, use exchange suffix as feedUrl
    const effectiveFeedUrl = formData.feed === 'YAHOO' && formData.yahooExchange
      ? formData.yahooExchange
      : formData.feedUrl;
    const effectiveLatestFeedUrl = formData.latestFeed === 'YAHOO' && formData.latestYahooExchange
      ? formData.latestYahooExchange
      : formData.latestFeedUrl;

    // Convert attributes and properties to Records
    const attributesRecord = entriesToRecord(attributes);
    const propertiesRecord = entriesToRecord(properties);

    try {
      if (isEditMode && security) {
        // Send all field values - empty string means "clear the field"
        const updateData: UpdateSecurityRequest = {
          name: formData.name || undefined,
          currency: formData.currency || undefined,
          targetCurrency: formData.targetCurrency || undefined,
          isin: formData.isin,      // send as-is to allow clearing
          wkn: formData.wkn,        // send as-is to allow clearing
          ticker: formData.ticker,  // send as-is to allow clearing
          feed: formData.feed,      // send as-is to allow clearing
          feedUrl: effectiveFeedUrl,// send as-is to allow clearing
          latestFeed: formData.latestFeed, // send as-is to allow clearing
          latestFeedUrl: effectiveLatestFeedUrl, // send as-is to allow clearing
          note: formData.note || undefined,
          isRetired: formData.isRetired,
          attributes: attributesRecord,
          properties: propertiesRecord,
        };
        await updateSecurity(security.id, updateData);
      } else {
        const createData: CreateSecurityRequest = {
          name: formData.name,
          currency: formData.currency,
          targetCurrency: formData.targetCurrency || undefined,
          isin: formData.isin || undefined,
          wkn: formData.wkn || undefined,
          ticker: formData.ticker || undefined,
          feed: formData.feed || undefined,
          feedUrl: effectiveFeedUrl || undefined,
          latestFeed: formData.latestFeed || undefined,
          latestFeedUrl: effectiveLatestFeedUrl || undefined,
          note: formData.note || undefined,
          attributes: attributesRecord,
          properties: propertiesRecord,
        };
        await createSecurity(createData);
      }
      onSuccess();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <h2 className="text-lg font-semibold">
            {isEditMode ? 'Wertpapier bearbeiten' : 'Neues Wertpapier'}
          </h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          {error && (
            <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
              {error}
            </div>
          )}

          {/* Name */}
          <div>
            <label className="block text-sm font-medium mb-1">
              Name <span className="text-destructive">*</span>
            </label>
            <input
              type="text"
              name="name"
              value={formData.name}
              onChange={handleChange}
              required
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder="z.B. Apple Inc."
            />
          </div>

          {/* Currency & Target Currency */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">
                Währung <span className="text-destructive">*</span>
              </label>
              <select
                name="currency"
                value={formData.currency}
                onChange={handleChange}
                required
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                {CURRENCIES.map((cur) => (
                  <option key={cur} value={cur}>
                    {cur}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Zielwährung</label>
              <select
                name="targetCurrency"
                value={formData.targetCurrency}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                <option value="">Keine</option>
                {CURRENCIES.map((cur) => (
                  <option key={cur} value={cur}>
                    {cur}
                  </option>
                ))}
              </select>
              <p className="text-xs text-muted-foreground mt-1">
                Für automatische Währungsumrechnung
              </p>
            </div>
          </div>

          {/* ISIN */}
          <div>
            <label className="block text-sm font-medium mb-1">ISIN</label>
            <input
              type="text"
              name="isin"
              value={formData.isin}
              onChange={handleChange}
              maxLength={12}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary font-mono"
              placeholder="z.B. US0378331005"
            />
            <p className="text-xs text-muted-foreground mt-1">
              12-stellige internationale Wertpapierkennung
            </p>
          </div>

          {/* WKN & Ticker in a row */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">WKN</label>
              <input
                type="text"
                name="wkn"
                value={formData.wkn}
                onChange={handleChange}
                maxLength={6}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary font-mono"
                placeholder="z.B. 865985"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Ticker</label>
              <input
                type="text"
                name="ticker"
                value={formData.ticker}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary font-mono"
                placeholder="z.B. AAPL"
              />
            </div>
          </div>

          {/* Provider Help Toggle */}
          <div className="border border-border rounded-md overflow-hidden">
            <button
              type="button"
              onClick={() => setShowProviderHelp(!showProviderHelp)}
              className="w-full flex items-center justify-between p-3 bg-muted/50 hover:bg-muted transition-colors text-left"
            >
              <div className="flex items-center gap-2">
                <HelpCircle size={16} className="text-primary" />
                <span className="text-sm font-medium">Welchen Kurslieferanten soll ich nutzen?</span>
              </div>
              {showProviderHelp ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
            </button>

            {showProviderHelp && (
              <div className="p-3 text-sm space-y-3 border-t border-border bg-card">
                <div className="grid gap-2">
                  <div className="flex gap-2">
                    <span className="font-semibold text-primary min-w-[120px]">Yahoo Finance</span>
                    <span className="text-muted-foreground">
                      Aktien, ETFs, Fonds weltweit. Kostenlos, zuverlässig. Empfohlen als Standard.
                    </span>
                  </div>
                  <div className="flex gap-2">
                    <span className="font-semibold text-orange-500 min-w-[120px]">Kraken</span>
                    <span className="text-muted-foreground">
                      Kryptowährungen (BTC, ETH, etc.). Direkte Börsenpreise, sehr genau.
                    </span>
                  </div>
                  <div className="flex gap-2">
                    <span className="font-semibold text-yellow-600 min-w-[120px]">CoinGecko</span>
                    <span className="text-muted-foreground">
                      Kryptowährungen (auch kleine Altcoins). Aggregierte Preise von vielen Börsen.
                    </span>
                  </div>
                  <div className="flex gap-2">
                    <span className="font-semibold text-blue-500 min-w-[120px]">Portfolio Report</span>
                    <span className="text-muted-foreground">
                      Deutsche Fonds, ETFs mit ISIN. Gute Alternative zu Yahoo für DE-Fonds.
                    </span>
                  </div>
                  <div className="flex gap-2">
                    <span className="font-semibold text-purple-500 min-w-[120px]">Alpha Vantage</span>
                    <span className="text-muted-foreground">
                      US-Aktien, Fundamentaldaten. API-Key erforderlich (kostenlos erhältlich).
                    </span>
                  </div>
                  {finnhubApiKey && (
                    <div className="flex gap-2">
                      <span className="font-semibold text-green-500 min-w-[120px]">Finnhub</span>
                      <span className="text-muted-foreground">
                        US-Aktien, Realtime-Daten. API-Key in Einstellungen konfiguriert.
                      </span>
                    </div>
                  )}
                  {twelveDataApiKey && (
                    <div className="flex gap-2">
                      <span className="font-semibold text-cyan-500 min-w-[120px]">Twelve Data</span>
                      <span className="text-muted-foreground">
                        Schweizer/EU-Aktien. API-Key in Einstellungen konfiguriert.
                      </span>
                    </div>
                  )}
                </div>

                <div className="pt-2 border-t border-border">
                  <p className="text-xs text-muted-foreground">
                    <strong>Empfehlung:</strong> Yahoo Finance für Aktien/ETFs, Kraken oder CoinGecko für Krypto.
                    Bei Schweizer Aktien ggf. Twelve Data nutzen (API-Key in Einstellungen).
                  </p>
                </div>
              </div>
            )}
          </div>

          {/* Feed Provider (Historical) */}
          <div>
            <label className="block text-sm font-medium mb-1">Kursquelle (Historisch)</label>
            <select
              name="feed"
              value={formData.feed}
              onChange={handleChange}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            >
              {feedProviders.map((provider) => (
                <option key={provider.value} value={provider.value}>
                  {provider.label}
                </option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground mt-1">
              Für historische Kurse und Charts
            </p>
          </div>

          {/* Yahoo Exchange Selection */}
          {formData.feed === 'YAHOO' && (
            <div>
              <label className="block text-sm font-medium mb-1">Börse (Yahoo)</label>
              <select
                name="yahooExchange"
                value={formData.yahooExchange}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                {YAHOO_EXCHANGES.map((exchange) => (
                  <option key={exchange.value} value={exchange.value}>
                    {exchange.label}
                  </option>
                ))}
              </select>
              <p className="text-xs text-muted-foreground mt-1">
                Börsen-Suffix wird an den Ticker angehängt (z.B. LIN.DE für Linde an XETRA)
              </p>
            </div>
          )}

          {/* CoinGecko hint */}
          {formData.feed === 'COINGECKO' && (
            <div className="p-3 bg-muted rounded-md">
              <p className="text-sm text-muted-foreground">
                <strong>CoinGecko:</strong> Ticker als Krypto-Symbol eingeben (z.B. BTC, ETH, SOL).
                Unterstützt: {CRYPTO_SYMBOLS.slice(0, 10).join(', ')}...
              </p>
            </div>
          )}

          {/* Kraken hint */}
          {formData.feed === 'KRAKEN' && (
            <div className="p-3 bg-muted rounded-md">
              <p className="text-sm text-muted-foreground">
                <strong>Kraken:</strong> Ticker als Krypto-Symbol eingeben (z.B. BTC, ETH, XRP).
                Preise werden direkt von der Kraken-Börse abgerufen (EUR-Paare).
              </p>
            </div>
          )}

          {/* Separator for current quote provider */}
          <div className="border-t border-border pt-4 mt-2">
            <h3 className="text-sm font-semibold text-muted-foreground mb-3">Aktueller Kurs (optional)</h3>
            <p className="text-xs text-muted-foreground mb-3">
              Falls abweichend vom historischen Kurslieferanten. Leer = gleicher Anbieter.
            </p>
          </div>

          {/* Latest Quote Provider */}
          <div>
            <label className="block text-sm font-medium mb-1">Kursquelle (Aktuell)</label>
            <select
              name="latestFeed"
              value={formData.latestFeed}
              onChange={handleChange}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            >
              <option value="">Wie historischer Kurs</option>
              {feedProviders.filter(p => p.value !== '').map((provider) => (
                <option key={provider.value} value={provider.value}>
                  {provider.label}
                </option>
              ))}
            </select>
          </div>

          {/* Yahoo Exchange Selection for Latest */}
          {formData.latestFeed === 'YAHOO' && (
            <div>
              <label className="block text-sm font-medium mb-1">Börse (Yahoo - Aktuell)</label>
              <select
                name="latestYahooExchange"
                value={formData.latestYahooExchange}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                {YAHOO_EXCHANGES.map((exchange) => (
                  <option key={exchange.value} value={exchange.value}>
                    {exchange.label}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Note */}
          <div>
            <label className="block text-sm font-medium mb-1">Notiz</label>
            <textarea
              name="note"
              value={formData.note}
              onChange={handleChange}
              rows={2}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary resize-none"
              placeholder="Optionale Notizen..."
            />
          </div>

          {/* Retired Flag (Edit mode only) */}
          {isEditMode && (
            <div className="flex items-center gap-3">
              <input
                type="checkbox"
                id="isRetired"
                checked={formData.isRetired}
                onChange={(e) => setFormData((prev) => ({ ...prev, isRetired: e.target.checked }))}
                className="w-4 h-4 rounded border-border focus:ring-2 focus:ring-primary"
              />
              <label htmlFor="isRetired" className="text-sm font-medium">
                Wertpapier stillgelegt
              </label>
              <span className="text-xs text-muted-foreground">
                (nicht mehr aktiv gehandelt)
              </span>
            </div>
          )}

          {/* Attributes & Properties (PP Round-Trip) */}
          <div className="border-t border-border pt-4 mt-2 space-y-3">
            <h3 className="text-sm font-semibold text-muted-foreground">Erweiterte Attribute (PP-kompatibel)</h3>

            <KeyValueEditor
              title="Attribute"
              entries={attributes}
              onChange={setAttributes}
              expanded={showAttributesExpanded}
              onToggleExpand={() => setShowAttributesExpanded(!showAttributesExpanded)}
            />

            <KeyValueEditor
              title="Eigenschaften"
              entries={properties}
              onChange={setProperties}
              expanded={showPropertiesExpanded}
              onToggleExpand={() => setShowPropertiesExpanded(!showPropertiesExpanded)}
            />

            <p className="text-xs text-muted-foreground">
              Diese Felder werden beim Export in PP-Dateien beibehalten.
            </p>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4 border-t border-border">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 border border-border rounded-md hover:bg-muted transition-colors"
            >
              Abbrechen
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !formData.name}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Speichern...' : isEditMode ? 'Speichern' : 'Erstellen'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
