import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import {
  LayoutDashboard,
  Briefcase,
  TrendingUp,
  Wallet,
  ArrowRightLeft,
  BarChart3,
  Settings,
  Menu,
  ChevronLeft,
  FileText,
  Plus,
  FolderOpen,
  Save,
  FilePlus,
  Database,
  RefreshCw,
} from 'lucide-react';
import './index.css';
import { AssetAllocationChart, SecurityPriceChart, MiniPortfolioChart } from './components/charts';

// Types for Portfolio Performance file - matches Rust protobuf structures
// Rust serde uses camelCase and flat arrays
interface PortfolioFile {
  version: number;
  baseCurrency: string;
  securities: Security[];
  accounts: Account[];
  portfolios: Portfolio[];
  watchlists: Watchlist[];
  taxonomies: Taxonomy[];
}

interface Security {
  uuid: string;
  name: string;
  currency: string;
  isin?: string | null;
  ticker?: string | null;
  wkn?: string | null;
  feed?: string | null;
  prices: PriceEntry[];
  latest?: LatestPrice | null;
}

interface PriceEntry {
  date: string;  // NaiveDate serialized as string
  value: number;  // price * 10^8
}

interface LatestPrice {
  date?: string | null;
  value?: number | null;
  high?: number | null;
  low?: number | null;
  volume?: number | null;
}

interface Account {
  uuid: string;
  name: string;
  currency: string;
  transactions: AccountTransaction[];
}

interface AccountTransaction {
  uuid: string;
  date: string;
  transactionType: string;
  amount: { amount: number; currency: string };
  shares?: number | null;
}

interface Portfolio {
  uuid: string;
  name: string;
  referenceAccountUuid?: string | null;
  transactions: PortfolioTransaction[];
}

interface PortfolioTransaction {
  uuid: string;
  date: string;
  transactionType: string;
  amount: { amount: number; currency: string };
  shares: number;
  securityUuid?: string | null;
}

interface Watchlist {
  name: string;
}

interface Taxonomy {
  id: string;
  name: string;
}

// ============================================================================
// Database Types (from Rust commands)
// ============================================================================

interface ImportResult {
  importId: number;
  filePath: string;
  version: number;
  baseCurrency: string;
  securitiesCount: number;
  accountsCount: number;
  portfoliosCount: number;
  transactionsCount: number;
  pricesCount: number;
  warnings: string[];
}

interface PortfolioData {
  id: number;
  uuid: string;
  name: string;
  referenceAccountName: string | null;
  isRetired: boolean;
  transactionsCount: number;
  holdingsCount: number;
}

// Portfolio holding for breakdown
interface PortfolioHolding {
  portfolioName: string;
  shares: number;
  value: number | null;
}

// Aggregated holding from backend (grouped by ISIN)
interface AggregatedHolding {
  isin: string;
  name: string;
  currency: string;
  totalShares: number;
  currentPrice: number | null;
  currentValue: number | null;
  costBasis: number;
  gainLoss: number | null;
  gainLossPercent: number | null;
  portfolios: PortfolioHolding[];
}

// Transaction data from database
interface TransactionData {
  id: number;
  uuid: string;
  ownerType: string;
  ownerName: string;
  txnType: string;
  date: string;
  amount: number;
  currency: string;
  shares: number | null;
  securityName: string | null;
  securityUuid: string | null;
  note: string | null;
  fees: number;
  taxes: number;
  hasForex: boolean;
}

// ============================================================================
// Portfolio Value Calculation
// ============================================================================

// PP stores prices and shares as value * 10^8
const PRICE_SCALE = 100_000_000;
const SHARES_SCALE = 100_000_000;

// Get latest price for a security (from latest element or last price in prices array)
function getLatestPrice(security: Security): number {
  // First try the latest element
  if (security.latest?.value) {
    return security.latest.value / PRICE_SCALE;
  }
  // Fallback to last price in prices array
  if (security.prices && security.prices.length > 0) {
    const prices = security.prices;
    // Sort by date and get the latest
    const sorted = [...prices].sort((a, b) => b.date.localeCompare(a.date));
    return sorted[0].value / PRICE_SCALE;
  }
  return 0;
}

// Transaction types that add shares
const BUY_TYPES = ['BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN'];
// Transaction types that remove shares
const SELL_TYPES = ['SELL', 'DELIVERY_OUTBOUND', 'TRANSFER_OUT'];

interface Holding {
  securityIndex: number;
  security: Security;
  shares: number;
  latestPrice: number;
  value: number;
  currency: string;
  portfolioName?: string;
}

// Grouped holding by ISIN for transparency
interface GroupedHolding {
  isin: string | null;
  name: string;  // Display name (from first security or aggregated)
  totalShares: number;
  totalValue: number;
  currency: string;
  latestPrice: number;
  // Individual holdings that make up this group
  holdings: Holding[];
  // Is this group expanded in the UI?
  isExpanded?: boolean;
}

// Calculate holdings from portfolio transactions (with portfolio context)
function calculateHoldings(portfolioFile: PortfolioFile): Holding[] {
  const securities = portfolioFile.securities || [];
  const portfolios = portfolioFile.portfolios || [];

  // Build UUID to index map for security lookup
  const securityIndexByUuid = new Map<string, number>();
  securities.forEach((sec, idx) => securityIndexByUuid.set(sec.uuid, idx));

  // Track shares per security UUID AND portfolio
  // Key: "securityUuid:portfolioName"
  const sharesPerSecurityPortfolio = new Map<string, { shares: number; portfolioName: string; securityUuid: string }>();

  // Debug: Count transaction types
  const txTypeCounts: Record<string, number> = {};

  for (const portfolio of portfolios) {
    const portfolioName = portfolio.name || 'Unbenannt';
    const transactions = portfolio.transactions || [];

    for (const tx of transactions) {
      // Count transaction types for debugging
      txTypeCounts[tx.transactionType] = (txTypeCounts[tx.transactionType] || 0) + 1;

      if (!tx.securityUuid) continue;

      const key = `${tx.securityUuid}:${portfolioName}`;
      const shares = (tx.shares || 0) / SHARES_SCALE;
      const current = sharesPerSecurityPortfolio.get(key) || { shares: 0, portfolioName, securityUuid: tx.securityUuid };

      if (BUY_TYPES.includes(tx.transactionType)) {
        current.shares += shares;
      } else if (SELL_TYPES.includes(tx.transactionType)) {
        current.shares -= shares;
      } else {
        // Log unhandled transaction types
        console.warn(`Unhandled transaction type: "${tx.transactionType}"`);
      }

      sharesPerSecurityPortfolio.set(key, current);
    }
  }

  // Debug output
  console.log('Transaction types found:', txTypeCounts);
  console.log('Expected BUY_TYPES:', BUY_TYPES);
  console.log('Expected SELL_TYPES:', SELL_TYPES);

  // Build holdings array
  const holdings: Holding[] = [];

  for (const [, data] of sharesPerSecurityPortfolio.entries()) {
    if (data.shares <= 0.0001) continue; // Skip if no shares held (or negligible)

    const securityIndex = securityIndexByUuid.get(data.securityUuid);
    if (securityIndex === undefined) continue;

    const security = securities[securityIndex];
    if (!security) continue;

    const latestPrice = getLatestPrice(security);
    const value = data.shares * latestPrice;

    holdings.push({
      securityIndex,
      security,
      shares: data.shares,
      latestPrice,
      value,
      currency: security.currency || 'EUR',
      portfolioName: data.portfolioName,
    });
  }

  // Sort by value descending
  holdings.sort((a, b) => b.value - a.value);

  return holdings;
}

// Group holdings by ISIN for aggregated view with drill-down
function groupHoldingsByISIN(holdings: Holding[]): GroupedHolding[] {
  const groups = new Map<string, GroupedHolding>();

  for (const holding of holdings) {
    // Use ISIN as key, or fallback to security name if no ISIN
    const key = holding.security.isin || `name:${holding.security.name}`;

    const existing = groups.get(key);
    if (existing) {
      existing.totalShares += holding.shares;
      existing.totalValue += holding.value;
      existing.holdings.push(holding);
      // Use highest price as representative (could also average)
      if (holding.latestPrice > existing.latestPrice) {
        existing.latestPrice = holding.latestPrice;
      }
    } else {
      groups.set(key, {
        isin: holding.security.isin || null,
        name: holding.security.name,
        totalShares: holding.shares,
        totalValue: holding.value,
        currency: holding.currency,
        latestPrice: holding.latestPrice,
        holdings: [holding],
      });
    }
  }

  // Convert to array and sort by total value
  const result = Array.from(groups.values());
  result.sort((a, b) => b.totalValue - a.totalValue);

  return result;
}

// Calculate total portfolio value
function calculateTotalValue(holdings: Holding[]): number {
  return holdings.reduce((sum, h) => sum + h.value, 0);
}

interface OpenResult {
  path: string;
  portfolio: PortfolioFile;
}

type View = 'dashboard' | 'portfolio' | 'securities' | 'accounts' | 'transactions' | 'reports' | 'settings';

interface NavItem {
  id: View;
  label: string;
  icon: React.ReactNode;
}

const navItems: NavItem[] = [
  { id: 'dashboard', label: 'Dashboard', icon: <LayoutDashboard className="w-5 h-5" /> },
  { id: 'portfolio', label: 'Portfolio', icon: <Briefcase className="w-5 h-5" /> },
  { id: 'securities', label: 'Wertpapiere', icon: <TrendingUp className="w-5 h-5" /> },
  { id: 'accounts', label: 'Konten', icon: <Wallet className="w-5 h-5" /> },
  { id: 'transactions', label: 'Buchungen', icon: <ArrowRightLeft className="w-5 h-5" /> },
  { id: 'reports', label: 'Berichte', icon: <BarChart3 className="w-5 h-5" /> },
];

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [portfolioFile, setPortfolioFile] = useState<PortfolioFile | null>(null);
  const [currentFilePath, setCurrentFilePath] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  // DB-based state
  const [dbPortfolios, setDbPortfolios] = useState<PortfolioData[]>([]);
  const [dbHoldings, setDbHoldings] = useState<AggregatedHolding[]>([]);
  const [dbPortfolioHistory, setDbPortfolioHistory] = useState<Array<{ date: string; value: number }>>([]);
  const [useDbData, setUseDbData] = useState(false);

  // Load holdings from database (aggregated by ISIN)
  const loadDbHoldings = useCallback(async () => {
    try {
      setIsLoading(true);

      // Get all portfolios for display
      const portfolios = await invoke<PortfolioData[]>('get_pp_portfolios', { importId: null });
      setDbPortfolios(portfolios);

      // Get aggregated holdings by ISIN
      const holdings = await invoke<AggregatedHolding[]>('get_all_holdings');
      setDbHoldings(holdings);

      // Get portfolio history for chart
      try {
        const history = await invoke<Array<{ date: string; value: number }>>('get_portfolio_history');
        setDbPortfolioHistory(history);
      } catch (historyErr) {
        console.warn('Could not load portfolio history:', historyErr);
      }

      setUseDbData(true);

    } catch (err) {
      setError(`Fehler beim Laden der Holdings: ${err}`);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Import portfolio file to database
  const handleImportToDb = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Portfolio Performance', extensions: ['portfolio'] },
        ],
      });

      if (selected) {
        setIsLoading(true);
        setError(null);

        const result = await invoke<ImportResult>('import_pp_file', { path: selected });
        console.log('Import result:', result);

        // Reload holdings
        await loadDbHoldings();
      }
    } catch (err) {
      setError(`Fehler beim Import: ${err}`);
    } finally {
      setIsLoading(false);
    }
  }, [loadDbHoldings]);

  const handleNewFile = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      const newPortfolio = await invoke<PortfolioFile>('create_new_portfolio', {
        baseCurrency: 'EUR',
      });
      setPortfolioFile(newPortfolio);
      setCurrentFilePath(null);
      setHasUnsavedChanges(true);
    } catch (err) {
      setError(`Fehler beim Erstellen: ${err}`);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const handleOpenFile = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Portfolio Performance', extensions: ['portfolio'] },
          { name: 'Alle Dateien', extensions: ['*'] },
        ],
      });

      if (selected) {
        setIsLoading(true);
        setError(null);
        const result = await invoke<OpenResult>('open_portfolio_file', {
          path: selected,
        });
        console.log('Loaded portfolio:', result);
        console.log('Securities count:', result.portfolio?.securities?.length);
        console.log('Accounts count:', result.portfolio?.accounts?.length);
        console.log('First security:', result.portfolio?.securities?.[0]);
        setPortfolioFile(result.portfolio);
        setCurrentFilePath(result.path);
        setHasUnsavedChanges(false);
      }
    } catch (err) {
      setError(`Fehler beim Öffnen: ${err}`);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const handleSaveFile = useCallback(async () => {
    if (!portfolioFile) return;

    try {
      let savePath = currentFilePath;

      if (!savePath) {
        const selected = await save({
          filters: [
            { name: 'Portfolio Performance', extensions: ['portfolio'] },
          ],
          defaultPath: 'portfolio.portfolio',
        });
        if (!selected) return;
        savePath = selected;
      }

      setIsLoading(true);
      setError(null);
      await invoke('save_portfolio_file', {
        path: savePath,
        portfolio: portfolioFile,
      });
      setCurrentFilePath(savePath);
      setHasUnsavedChanges(false);
    } catch (err) {
      setError(`Fehler beim Speichern: ${err}`);
    } finally {
      setIsLoading(false);
    }
  }, [portfolioFile, currentFilePath]);

  const handleSaveAsFile = useCallback(async () => {
    if (!portfolioFile) return;

    try {
      const selected = await save({
        filters: [
          { name: 'Portfolio Performance', extensions: ['portfolio'] },
        ],
        defaultPath: currentFilePath || 'portfolio.portfolio',
      });

      if (selected) {
        setIsLoading(true);
        setError(null);
        await invoke('save_portfolio_file', {
          path: selected,
          portfolio: portfolioFile,
        });
        setCurrentFilePath(selected);
        setHasUnsavedChanges(false);
      }
    } catch (err) {
      setError(`Fehler beim Speichern: ${err}`);
    } finally {
      setIsLoading(false);
    }
  }, [portfolioFile, currentFilePath]);

  const fileName = currentFilePath
    ? currentFilePath.split('/').pop() || 'Portfolio'
    : portfolioFile
    ? 'Neues Portfolio'
    : null;

  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <aside
        className={`${
          sidebarCollapsed ? 'w-16' : 'w-64'
        } flex flex-col border-r border-border bg-card transition-all duration-300`}
      >
        {/* Logo/Header */}
        <div className="flex items-center justify-between h-14 px-4 border-b border-border">
          {!sidebarCollapsed && (
            <div className="flex items-center gap-2">
              <TrendingUp className="w-6 h-6 text-primary" />
              <span className="font-semibold text-foreground">Portfolio</span>
            </div>
          )}
          <button
            onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
            className="p-1.5 rounded-md hover:bg-accent transition-colors"
          >
            {sidebarCollapsed ? (
              <Menu className="w-5 h-5 text-muted-foreground" />
            ) : (
              <ChevronLeft className="w-5 h-5 text-muted-foreground" />
            )}
          </button>
        </div>

        {/* Navigation */}
        <nav className="flex-1 p-2 space-y-1">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => setCurrentView(item.id)}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-md transition-colors ${
                currentView === item.id
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
              }`}
            >
              {item.icon}
              {!sidebarCollapsed && <span>{item.label}</span>}
            </button>
          ))}
        </nav>

        {/* Settings at bottom */}
        <div className="p-2 border-t border-border">
          <button
            onClick={() => setCurrentView('settings')}
            className={`w-full flex items-center gap-3 px-3 py-2 rounded-md transition-colors ${
              currentView === 'settings'
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
            }`}
          >
            <Settings className="w-5 h-5" />
            {!sidebarCollapsed && <span>Einstellungen</span>}
          </button>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <header className="h-14 flex items-center justify-between px-6 border-b border-border bg-card">
          <div className="flex items-center gap-4">
            <h1 className="text-lg font-semibold text-foreground">
              {navItems.find((item) => item.id === currentView)?.label || 'Einstellungen'}
            </h1>
            {fileName && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <FileText className="w-4 h-4" />
                <span>{fileName}</span>
                {hasUnsavedChanges && <span className="text-yellow-500">*</span>}
              </div>
            )}
          </div>
          <div className="flex items-center gap-2">
            {/* File operations */}
            <button
              onClick={handleNewFile}
              disabled={isLoading}
              className="flex items-center gap-2 px-3 py-1.5 text-sm border border-input rounded-md hover:bg-accent transition-colors disabled:opacity-50"
              title="Neue Datei"
            >
              <FilePlus className="w-4 h-4" />
              Neu
            </button>
            <button
              onClick={handleOpenFile}
              disabled={isLoading}
              className="flex items-center gap-2 px-3 py-1.5 text-sm border border-input rounded-md hover:bg-accent transition-colors disabled:opacity-50"
              title="Datei öffnen"
            >
              <FolderOpen className="w-4 h-4" />
              Öffnen
            </button>
            {portfolioFile && (
              <>
                <button
                  onClick={handleSaveFile}
                  disabled={isLoading || !hasUnsavedChanges}
                  className="flex items-center gap-2 px-3 py-1.5 text-sm border border-input rounded-md hover:bg-accent transition-colors disabled:opacity-50"
                  title="Speichern"
                >
                  <Save className="w-4 h-4" />
                  Speichern
                </button>
                <button
                  onClick={handleSaveAsFile}
                  disabled={isLoading}
                  className="flex items-center gap-2 px-3 py-1.5 text-sm border border-input rounded-md hover:bg-accent transition-colors disabled:opacity-50"
                  title="Speichern unter..."
                >
                  Speichern als...
                </button>
              </>
            )}
            <div className="w-px h-6 bg-border mx-2" />
            {/* DB Import Controls */}
            <button
              onClick={handleImportToDb}
              disabled={isLoading}
              className="flex items-center gap-2 px-3 py-1.5 text-sm bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors disabled:opacity-50"
              title="Portfolio in Datenbank importieren"
            >
              <Database className="w-4 h-4" />
              Import DB
            </button>
            {useDbData && (
              <button
                onClick={loadDbHoldings}
                disabled={isLoading}
                className="flex items-center gap-2 px-3 py-1.5 text-sm border border-input rounded-md hover:bg-accent transition-colors disabled:opacity-50"
                title="Holdings neu laden"
              >
                <RefreshCw className={`w-4 h-4 ${isLoading ? 'animate-spin' : ''}`} />
              </button>
            )}
            <div className="w-px h-6 bg-border mx-2" />
            <button
              disabled={!portfolioFile}
              className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              <Plus className="w-4 h-4" />
              Neue Buchung
            </button>
          </div>
        </header>

        {/* Error Banner */}
        {error && (
          <div className="px-6 py-3 bg-red-100 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800 text-red-800 dark:text-red-200 text-sm flex items-center justify-between">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="text-red-600 hover:text-red-800">
              Schließen
            </button>
          </div>
        )}

        {/* Loading Indicator */}
        {isLoading && (
          <div className="px-6 py-2 bg-blue-50 dark:bg-blue-900/20 border-b border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-200 text-sm">
            Lädt...
          </div>
        )}

        {/* Content Area */}
        <div className="flex-1 overflow-auto p-6">
          <ContentView
            view={currentView}
            portfolioFile={portfolioFile}
            onOpenFile={handleOpenFile}
            onImportToDb={handleImportToDb}
            dbHoldings={dbHoldings}
            dbPortfolios={dbPortfolios}
            dbPortfolioHistory={dbPortfolioHistory}
            useDbData={useDbData}
          />
        </div>
      </main>
    </div>
  );
}

interface ContentViewProps {
  view: View;
  portfolioFile: PortfolioFile | null;
  onOpenFile: () => void;
  onImportToDb: () => void;
  dbHoldings: AggregatedHolding[];
  dbPortfolios: PortfolioData[];
  dbPortfolioHistory: Array<{ date: string; value: number }>;
  useDbData: boolean;
}

function ContentView({ view, portfolioFile, onOpenFile, onImportToDb, dbHoldings, dbPortfolios, dbPortfolioHistory, useDbData }: ContentViewProps) {
  switch (view) {
    case 'dashboard':
      return <DashboardView portfolioFile={portfolioFile} onOpenFile={onOpenFile} onImportToDb={onImportToDb} dbHoldings={dbHoldings} dbPortfolios={dbPortfolios} dbPortfolioHistory={dbPortfolioHistory} useDbData={useDbData} />;
    case 'portfolio':
      return <PortfolioView portfolioFile={portfolioFile} dbPortfolios={dbPortfolios} useDbData={useDbData} />;
    case 'securities':
      return <SecuritiesView portfolioFile={portfolioFile} />;
    case 'accounts':
      return <AccountsView portfolioFile={portfolioFile} />;
    case 'transactions':
      return <TransactionsView portfolioFile={portfolioFile} useDbData={useDbData} />;
    case 'reports':
      return <ReportsView />;
    case 'settings':
      return <SettingsView />;
    default:
      return <DashboardView portfolioFile={portfolioFile} onOpenFile={onOpenFile} onImportToDb={onImportToDb} dbHoldings={dbHoldings} dbPortfolios={dbPortfolios} dbPortfolioHistory={dbPortfolioHistory} useDbData={useDbData} />;
  }
}

interface DashboardProps {
  portfolioFile: PortfolioFile | null;
  onOpenFile: () => void;
  onImportToDb: () => void;
  dbHoldings: AggregatedHolding[];
  dbPortfolios: PortfolioData[];
  dbPortfolioHistory: Array<{ date: string; value: number }>;
  useDbData: boolean;
}

function DashboardView({ portfolioFile, onOpenFile, onImportToDb, dbHoldings, dbPortfolios, dbPortfolioHistory, useDbData }: DashboardProps) {
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set());

  const toggleGroup = (key: string) => {
    setExpandedGroups(prev => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  // Show DB-based holdings if available (aggregated by ISIN from backend)
  if (useDbData && dbHoldings.length > 0) {
    const totalValue = dbHoldings.reduce((sum, h) => sum + (h.currentValue || 0), 0);
    const totalCostBasis = dbHoldings.reduce((sum, h) => sum + h.costBasis, 0);
    const totalGainLoss = totalValue - totalCostBasis;
    const totalGainLossPercent = totalCostBasis > 0 ? (totalGainLoss / totalCostBasis) * 100 : 0;

    return (
      <div className="space-y-6">
        {/* Portfolio Value Header with Mini Chart */}
        <div className="bg-card rounded-lg border border-border p-6">
          <div className="flex gap-6">
            {/* Left: Value Info */}
            <div className="flex-1">
              <div className="flex items-center gap-2 mb-1">
                <Database className="w-4 h-4 text-green-600" />
                <span className="text-sm text-green-600 font-medium">Aus Datenbank (ISIN-aggregiert)</span>
              </div>
              <div className="text-sm text-muted-foreground mb-1">Depotwert</div>
              <div className="text-3xl font-bold">
                {totalValue.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR
              </div>
              <div className="flex items-center gap-4 mt-2">
                <div className="text-sm text-muted-foreground">
                  {dbHoldings.length} Positionen
                </div>
                <div className={`text-sm ${totalGainLoss >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                  {totalGainLoss >= 0 ? '+' : ''}{totalGainLoss.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR
                  ({totalGainLossPercent >= 0 ? '+' : ''}{totalGainLossPercent.toFixed(2)}%)
                </div>
              </div>
            </div>
            {/* Right: Mini Year Chart */}
            {dbPortfolioHistory.length > 0 && (
              <div className="w-64 flex flex-col justify-center">
                <div className="text-xs text-muted-foreground mb-1">Jahresentwicklung</div>
                <MiniPortfolioChart data={dbPortfolioHistory} height={70} />
              </div>
            )}
          </div>
        </div>

        {/* Summary Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <SummaryCard title="Positionen" value={String(dbHoldings.length)} change="" positive />
          <SummaryCard title="Portfolios" value={String(dbPortfolios.filter(p => !p.isRetired).length)} change="" positive />
          <SummaryCard title="Einstandswert" value={`${totalCostBasis.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR`} change="" positive />
          <SummaryCard
            title="Gewinn/Verlust"
            value={`${totalGainLoss >= 0 ? '+' : ''}${totalGainLoss.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR`}
            change={`${totalGainLossPercent >= 0 ? '+' : ''}${totalGainLossPercent.toFixed(2)}%`}
            positive={totalGainLoss >= 0}
          />
        </div>

        {/* DB Holdings Table */}
        <div className="bg-card rounded-lg border border-border p-6">
          <h2 className="text-lg font-semibold mb-4">Depot ({dbHoldings.length} Positionen)</h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="w-8"></th>
                  <th className="text-left py-2 font-medium">Wertpapier</th>
                  <th className="text-left py-2 font-medium">ISIN</th>
                  <th className="text-right py-2 font-medium">Bestand</th>
                  <th className="text-right py-2 font-medium">Kurs</th>
                  <th className="text-right py-2 font-medium">Wert</th>
                  <th className="text-right py-2 font-medium">Gewinn</th>
                  <th className="text-right py-2 font-medium">Anteil</th>
                </tr>
              </thead>
              <tbody>
                {dbHoldings.map((holding) => {
                  const value = holding.currentValue || 0;
                  const gainLoss = holding.gainLoss || 0;
                  const gainLossPercent = holding.gainLossPercent || 0;
                  const hasMultiplePortfolios = holding.portfolios.length > 1;
                  const isExpanded = expandedGroups.has(holding.isin);

                  return (
                    <>
                      <tr
                        key={holding.isin}
                        className={`border-b border-border hover:bg-accent/30 ${hasMultiplePortfolios ? 'cursor-pointer' : ''}`}
                        onClick={() => hasMultiplePortfolios && toggleGroup(holding.isin)}
                      >
                        <td className="py-2 text-center">
                          {hasMultiplePortfolios && (
                            <span className="text-muted-foreground text-xs">
                              {isExpanded ? '▼' : '▶'}
                            </span>
                          )}
                        </td>
                        <td className="py-2">
                          <div className="font-medium">{holding.name}</div>
                          <div className="text-xs text-muted-foreground">
                            {holding.currency}
                            {hasMultiplePortfolios && (
                              <span className="ml-2 px-1.5 py-0.5 bg-accent rounded text-xs">
                                {holding.portfolios.length} Depots
                              </span>
                            )}
                          </div>
                        </td>
                        <td className="py-2 text-xs text-muted-foreground font-mono">{holding.isin}</td>
                        <td className="py-2 text-right font-medium">{holding.totalShares.toLocaleString('de-DE', { maximumFractionDigits: 6 })}</td>
                        <td className="py-2 text-right">
                          {holding.currentPrice
                            ? `${holding.currentPrice.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${holding.currency}`
                            : '-'}
                        </td>
                        <td className="py-2 text-right font-medium">
                          {value.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR
                        </td>
                        <td className={`py-2 text-right ${gainLoss >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                          {gainLoss >= 0 ? '+' : ''}{gainLoss.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR
                          <div className="text-xs">({gainLossPercent >= 0 ? '+' : ''}{gainLossPercent.toFixed(2)}%)</div>
                        </td>
                        <td className="py-2 text-right text-muted-foreground">
                          {totalValue > 0 ? ((value / totalValue) * 100).toFixed(2) : '0.00'}%
                        </td>
                      </tr>
                      {/* Portfolio breakdown rows */}
                      {isExpanded && holding.portfolios.map((pf, idx) => (
                        <tr
                          key={`${holding.isin}-${idx}`}
                          className="bg-accent/20 border-b border-border/50"
                        >
                          <td className="py-1.5"></td>
                          <td className="py-1.5 pl-4" colSpan={2}>
                            <div className="flex items-center gap-2 text-sm">
                              <Briefcase className="w-3 h-3 text-muted-foreground" />
                              <span className="text-muted-foreground">{pf.portfolioName}</span>
                            </div>
                          </td>
                          <td className="py-1.5 text-right text-muted-foreground">
                            {pf.shares.toLocaleString('de-DE', { maximumFractionDigits: 6 })}
                          </td>
                          <td className="py-1.5"></td>
                          <td className="py-1.5 text-right text-muted-foreground">
                            {pf.value !== null ? pf.value.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + ' EUR' : '-'}
                          </td>
                          <td className="py-1.5"></td>
                          <td className="py-1.5 text-right text-muted-foreground/70">
                            {totalValue > 0 && pf.value !== null ? (((pf.value) / totalValue) * 100).toFixed(2) + '%' : '-'}
                          </td>
                        </tr>
                      ))}
                    </>
                  );
                })}
              </tbody>
              <tfoot>
                <tr className="font-semibold">
                  <td className="py-2"></td>
                  <td className="py-2">Gesamt</td>
                  <td></td>
                  <td></td>
                  <td></td>
                  <td className="py-2 text-right">{totalValue.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR</td>
                  <td className={`py-2 text-right ${totalGainLoss >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {totalGainLoss >= 0 ? '+' : ''}{totalGainLoss.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} EUR
                  </td>
                  <td className="py-2 text-right">100%</td>
                </tr>
              </tfoot>
            </table>
          </div>
        </div>

        {/* Portfolios Overview */}
        <div className="bg-card rounded-lg border border-border p-6">
          <h2 className="text-lg font-semibold mb-4">
            Portfolios ({dbPortfolios.filter(p => !p.isRetired).length} aktiv, {dbPortfolios.filter(p => p.isRetired).length} inaktiv)
          </h2>
          <div className="space-y-2">
            {/* Active portfolios first */}
            {dbPortfolios.filter(p => !p.isRetired).map((portfolio) => (
              <div key={portfolio.uuid} className="flex items-center justify-between py-2 px-3 rounded-md bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800">
                <div className="flex items-center gap-2">
                  <span className="w-2 h-2 rounded-full bg-green-500"></span>
                  <span className="font-medium">{portfolio.name}</span>
                </div>
                <div className="text-sm text-muted-foreground">
                  {portfolio.transactionsCount} Buchungen | {portfolio.holdingsCount} Positionen
                </div>
              </div>
            ))}
            {/* Inactive portfolios */}
            {dbPortfolios.filter(p => p.isRetired).map((portfolio) => (
              <div key={portfolio.uuid} className="flex items-center justify-between py-2 px-3 rounded-md bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700 opacity-60">
                <div className="flex items-center gap-2">
                  <span className="w-2 h-2 rounded-full bg-gray-400"></span>
                  <span className="font-medium">{portfolio.name}</span>
                  <span className="text-xs px-1.5 py-0.5 bg-gray-200 dark:bg-gray-700 rounded">inaktiv</span>
                </div>
                <div className="text-sm text-muted-foreground">
                  {portfolio.transactionsCount} Buchungen | {portfolio.holdingsCount} Positionen
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (!portfolioFile) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center">
        <TrendingUp className="w-16 h-16 text-muted-foreground mb-4" />
        <h2 className="text-2xl font-semibold mb-2">Willkommen bei Portfolio Performance</h2>
        <p className="text-muted-foreground mb-6 max-w-md">
          Öffnen Sie eine bestehende Portfolio Performance Datei oder importieren Sie sie in die Datenbank.
        </p>
        <div className="flex gap-4">
          <button
            onClick={onOpenFile}
            className="flex items-center gap-2 px-4 py-2 border border-input rounded-md hover:bg-accent transition-colors"
          >
            <FolderOpen className="w-5 h-5" />
            Portfolio öffnen
          </button>
          <button
            onClick={onImportToDb}
            className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors"
          >
            <Database className="w-5 h-5" />
            In DB importieren
          </button>
        </div>
      </div>
    );
  }

  const securitiesCount = portfolioFile.securities?.length || 0;
  const accountsCount = portfolioFile.accounts?.length || 0;
  const securities = portfolioFile.securities || [];

  // Calculate actual holdings from transactions
  const holdings = calculateHoldings(portfolioFile);
  const totalValue = calculateTotalValue(holdings);

  // Group holdings by ISIN for aggregated view
  const groupedHoldings = groupHoldingsByISIN(holdings);
  const uniquePositionsCount = groupedHoldings.length;

  // Find a security with price history for the detail chart
  const securityWithPrices = holdings.length > 0
    ? holdings.find(h => h.security.prices && h.security.prices.length > 5)?.security
    : securities.find(s => s.prices && s.prices.length > 5);

  return (
    <div className="space-y-6">
      {/* Portfolio Value Header */}
      <div className="bg-card rounded-lg border border-border p-6">
        <div className="text-sm text-muted-foreground mb-1">Depotwert</div>
        <div className="text-3xl font-bold">
          {totalValue.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} {portfolioFile.baseCurrency}
        </div>
        <div className="text-sm text-muted-foreground mt-1">
          {uniquePositionsCount} Positionen {holdings.length !== uniquePositionsCount && `(${holdings.length} Einzelpositionen)`}
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <SummaryCard title="Positionen" value={String(uniquePositionsCount)} change="" positive />
        <SummaryCard title="Wertpapiere" value={String(securitiesCount)} change="" positive />
        <SummaryCard title="Konten" value={String(accountsCount)} change="" positive />
        <SummaryCard title="Basiswährung" value={portfolioFile.baseCurrency} change="" positive />
      </div>

      {/* Holdings Table - Grouped by ISIN with expandable rows */}
      {groupedHoldings.length > 0 && (
        <div className="bg-card rounded-lg border border-border p-6">
          <h2 className="text-lg font-semibold mb-4">Depot ({uniquePositionsCount} Positionen)</h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-2 font-medium w-8"></th>
                  <th className="text-left py-2 font-medium">Wertpapier</th>
                  <th className="text-right py-2 font-medium">Bestand</th>
                  <th className="text-right py-2 font-medium">Kurs</th>
                  <th className="text-right py-2 font-medium">Wert</th>
                  <th className="text-right py-2 font-medium">Anteil</th>
                </tr>
              </thead>
              <tbody>
                {groupedHoldings.map((group) => {
                  const groupKey = group.isin || group.name;
                  const isExpanded = expandedGroups.has(groupKey);
                  const hasMultiple = group.holdings.length > 1;

                  return (
                    <>
                      {/* Main aggregated row */}
                      <tr
                        key={groupKey}
                        className={`border-b border-border ${hasMultiple ? 'cursor-pointer hover:bg-accent/50' : ''}`}
                        onClick={() => hasMultiple && toggleGroup(groupKey)}
                      >
                        <td className="py-2 text-center">
                          {hasMultiple && (
                            <span className="text-muted-foreground">
                              {isExpanded ? '▼' : '▶'}
                            </span>
                          )}
                        </td>
                        <td className="py-2">
                          <div className="font-medium">{group.name}</div>
                          <div className="text-xs text-muted-foreground">
                            {group.isin || ''}
                            {hasMultiple && (
                              <span className="ml-2 px-1.5 py-0.5 bg-accent rounded text-xs">
                                {group.holdings.length} Positionen
                              </span>
                            )}
                          </div>
                        </td>
                        <td className="py-2 text-right font-medium">{group.totalShares.toLocaleString('de-DE', { maximumFractionDigits: 4 })}</td>
                        <td className="py-2 text-right">{group.latestPrice.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} {group.currency}</td>
                        <td className="py-2 text-right font-medium">{group.totalValue.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} {group.currency}</td>
                        <td className="py-2 text-right text-muted-foreground">{((group.totalValue / totalValue) * 100).toFixed(2)}%</td>
                      </tr>

                      {/* Expanded detail rows */}
                      {isExpanded && group.holdings.map((holding, idx) => (
                        <tr
                          key={`${groupKey}-${idx}`}
                          className="bg-accent/30 border-b border-border/50"
                        >
                          <td className="py-1.5"></td>
                          <td className="py-1.5 pl-6">
                            <div className="text-sm text-muted-foreground">
                              <span className="font-medium">{holding.portfolioName}</span>
                              <span className="mx-2">·</span>
                              <span>{holding.security.name}</span>
                            </div>
                          </td>
                          <td className="py-1.5 text-right text-muted-foreground">{holding.shares.toLocaleString('de-DE', { maximumFractionDigits: 4 })}</td>
                          <td className="py-1.5 text-right text-muted-foreground">{holding.latestPrice.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} {holding.currency}</td>
                          <td className="py-1.5 text-right text-muted-foreground">{holding.value.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} {holding.currency}</td>
                          <td className="py-1.5 text-right text-muted-foreground/70">{((holding.value / totalValue) * 100).toFixed(2)}%</td>
                        </tr>
                      ))}
                    </>
                  );
                })}
              </tbody>
              <tfoot>
                <tr className="font-semibold">
                  <td className="py-2"></td>
                  <td className="py-2">Gesamt</td>
                  <td></td>
                  <td></td>
                  <td className="py-2 text-right">{totalValue.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} {portfolioFile.baseCurrency}</td>
                  <td className="py-2 text-right">100%</td>
                </tr>
              </tfoot>
            </table>
          </div>
        </div>
      )}

      {/* Asset Allocation Chart */}
      {holdings.length > 0 && (
        <div className="bg-card rounded-lg border border-border p-6">
          <h2 className="text-lg font-semibold mb-4">Währungsverteilung</h2>
          <AssetAllocationChart securities={holdings.map(h => h.security)} groupBy="currency" />
        </div>
      )}

      {/* Security Price Chart (if available) */}
      {securityWithPrices && securityWithPrices.prices && securityWithPrices.prices.length > 0 && (
        <div className="bg-card rounded-lg border border-border p-6">
          <h2 className="text-lg font-semibold mb-4">Kursentwicklung</h2>
          <SecurityPriceChart
            prices={securityWithPrices.prices}
            currency={securityWithPrices.currency}
            name={securityWithPrices.name}
          />
        </div>
      )}

      {/* All Securities Overview (collapsed) */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Alle Wertpapiere ({securitiesCount})</h2>
        {securitiesCount > 0 ? (
          <div className="space-y-3">
            {portfolioFile.securities.slice(0, 5).map((security, index) => (
              <div key={security.uuid || `security-${index}`} className="flex items-center justify-between py-2 border-b border-border last:border-0">
                <div>
                  <div className="font-medium">{security.name || 'Unbekannt'}</div>
                  <div className="text-sm text-muted-foreground">
                    {security.isin || security.ticker || security.wkn || 'Keine Kennung'}
                  </div>
                </div>
                <div className="text-sm text-muted-foreground">{security.currency}</div>
              </div>
            ))}
            {securitiesCount > 5 && (
              <div className="text-sm text-muted-foreground text-center pt-2">
                ... und {securitiesCount - 5} weitere
              </div>
            )}
          </div>
        ) : (
          <div className="text-muted-foreground text-sm">
            Keine Wertpapiere vorhanden.
          </div>
        )}
      </div>

      {/* Accounts Overview */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Konten ({accountsCount})</h2>
        {accountsCount > 0 ? (
          <div className="space-y-3">
            {portfolioFile.accounts.map((account, index) => (
              <div key={account.uuid || `account-${index}`} className="flex items-center justify-between py-2 border-b border-border last:border-0">
                <div className="font-medium">{account.name || 'Unbenannt'}</div>
                <div className="text-sm text-muted-foreground">
                  {account.transactions?.length || 0} Buchungen
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-muted-foreground text-sm">
            Keine Konten vorhanden.
          </div>
        )}
      </div>
    </div>
  );
}

function SummaryCard({
  title,
  value,
  change,
  positive,
}: {
  title: string;
  value: string;
  change: string;
  positive: boolean;
}) {
  return (
    <div className="bg-card rounded-lg border border-border p-4">
      <div className="text-sm text-muted-foreground mb-1">{title}</div>
      <div className="text-2xl font-semibold">{value}</div>
      {change && (
        <div className={`text-sm mt-1 ${positive ? 'text-green-500' : 'text-red-500'}`}>
          {change}
        </div>
      )}
    </div>
  );
}

function PortfolioView({ portfolioFile, dbPortfolios, useDbData }: { portfolioFile: PortfolioFile | null; dbPortfolios: PortfolioData[]; useDbData: boolean }) {
  // Show DB portfolios if available
  if (useDbData && dbPortfolios.length > 0) {
    return (
      <div className="space-y-4">
        <div className="flex items-center gap-2 text-sm text-green-600 mb-2">
          <Database className="w-4 h-4" />
          <span className="font-medium">Aus Datenbank</span>
        </div>
        {dbPortfolios.filter(p => !p.isRetired).map((portfolio) => (
          <div key={portfolio.uuid} className="bg-card rounded-lg border border-border p-6">
            <h2 className="text-lg font-semibold mb-2">{portfolio.name}</h2>
            <div className="grid grid-cols-2 gap-4 text-sm text-muted-foreground">
              <div>
                <span className="font-medium">{portfolio.transactionsCount}</span> Transaktionen
              </div>
              <div>
                <span className="font-medium">{portfolio.holdingsCount}</span> Positionen
              </div>
              {portfolio.referenceAccountName && (
                <div className="col-span-2">
                  Referenzkonto: {portfolio.referenceAccountName}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (!portfolioFile) {
    return (
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Portfolio</h2>
        <p className="text-muted-foreground">
          Öffnen Sie eine Portfolio Performance Datei, um Portfolios anzuzeigen.
        </p>
      </div>
    );
  }

  const portfolios = portfolioFile.portfolios || [];

  return (
    <div className="space-y-4">
      {portfolios.length > 0 ? (
        portfolios.map((portfolio, index) => (
          <div key={portfolio.uuid || `portfolio-${index}`} className="bg-card rounded-lg border border-border p-6">
            <h2 className="text-lg font-semibold mb-2">{portfolio.name || 'Unbenannt'}</h2>
            <p className="text-sm text-muted-foreground">
              {portfolio.transactions?.length || 0} Transaktionen
            </p>
          </div>
        ))
      ) : (
        <div className="bg-card rounded-lg border border-border p-6">
          <h2 className="text-lg font-semibold mb-4">Portfolio</h2>
          <p className="text-muted-foreground">Keine Portfolios vorhanden.</p>
        </div>
      )}
    </div>
  );
}

function SecuritiesView({ portfolioFile }: { portfolioFile: PortfolioFile | null }) {
  if (!portfolioFile) {
    return (
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Wertpapiere</h2>
        <p className="text-muted-foreground">
          Öffnen Sie eine Portfolio Performance Datei, um Wertpapiere anzuzeigen.
        </p>
      </div>
    );
  }

  const securities = portfolioFile.securities || [];

  return (
    <div className="bg-card rounded-lg border border-border p-6">
      <h2 className="text-lg font-semibold mb-4">Wertpapiere ({securities.length})</h2>
      {securities.length > 0 ? (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left py-2 font-medium">Name</th>
                <th className="text-left py-2 font-medium">ISIN</th>
                <th className="text-left py-2 font-medium">Ticker</th>
                <th className="text-left py-2 font-medium">Währung</th>
              </tr>
            </thead>
            <tbody>
              {securities.map((security, index) => (
                <tr key={security.uuid || `sec-${index}`} className="border-b border-border last:border-0">
                  <td className="py-2">{security.name || 'Unbekannt'}</td>
                  <td className="py-2 text-muted-foreground">{security.isin || '-'}</td>
                  <td className="py-2 text-muted-foreground">{security.ticker || '-'}</td>
                  <td className="py-2 text-muted-foreground">{security.currency}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <p className="text-muted-foreground">Keine Wertpapiere vorhanden.</p>
      )}
    </div>
  );
}

function AccountsView({ portfolioFile }: { portfolioFile: PortfolioFile | null }) {
  if (!portfolioFile) {
    return (
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Konten</h2>
        <p className="text-muted-foreground">
          Öffnen Sie eine Portfolio Performance Datei, um Konten anzuzeigen.
        </p>
      </div>
    );
  }

  const accounts = portfolioFile.accounts || [];

  return (
    <div className="space-y-4">
      {accounts.length > 0 ? (
        accounts.map((account, index) => (
          <div key={account.uuid || `acc-${index}`} className="bg-card rounded-lg border border-border p-6">
            <h2 className="text-lg font-semibold mb-2">{account.name || 'Unbenannt'}</h2>
            <p className="text-sm text-muted-foreground">
              Währung: {account.currency} | {account.transactions?.length || 0} Buchungen
            </p>
          </div>
        ))
      ) : (
        <div className="bg-card rounded-lg border border-border p-6">
          <h2 className="text-lg font-semibold mb-4">Konten</h2>
          <p className="text-muted-foreground">Keine Konten vorhanden.</p>
        </div>
      )}
    </div>
  );
}

function TransactionsView({ portfolioFile, useDbData }: { portfolioFile: PortfolioFile | null; useDbData: boolean }) {
  const [dbTransactions, setDbTransactions] = useState<TransactionData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [filterOwnerType, setFilterOwnerType] = useState<string>('all');
  const [filterTxnType, setFilterTxnType] = useState<string>('all');
  const [displayLimit, setDisplayLimit] = useState(100);

  // Load transactions from database
  useEffect(() => {
    if (!useDbData) return;

    const loadTransactions = async () => {
      setIsLoading(true);
      try {
        const transactions = await invoke<TransactionData[]>('get_transactions', {
          ownerType: null,
          ownerId: null,
          securityId: null,
          limit: 2000,
          offset: null,
        });
        setDbTransactions(transactions);
      } catch (err) {
        console.error('Failed to load transactions:', err);
      } finally {
        setIsLoading(false);
      }
    };

    loadTransactions();
  }, [useDbData]);

  // Transaction type labels
  const txnTypeLabels: Record<string, string> = {
    'BUY': 'Kauf',
    'SELL': 'Verkauf',
    'DELIVERY_INBOUND': 'Einlieferung',
    'DELIVERY_OUTBOUND': 'Auslieferung',
    'TRANSFER_IN': 'Umbuchung (Eingang)',
    'TRANSFER_OUT': 'Umbuchung (Ausgang)',
    'DIVIDENDS': 'Dividende',
    'INTEREST': 'Zinsen',
    'INTEREST_CHARGE': 'Zinsbelastung',
    'DEPOSIT': 'Einlage',
    'REMOVAL': 'Entnahme',
    'FEES': 'Gebühren',
    'FEES_REFUND': 'Gebührenerstattung',
    'TAXES': 'Steuern',
    'TAX_REFUND': 'Steuererstattung',
  };

  // Show DB-based transactions
  if (useDbData) {
    // Filter transactions by type
    const filteredTransactions = dbTransactions.filter(tx => {
      if (filterOwnerType !== 'all' && tx.ownerType !== filterOwnerType) return false;
      if (filterTxnType !== 'all' && tx.txnType !== filterTxnType) return false;
      return true;
    });

    // Get unique transaction types for filter
    const uniqueTxnTypes = [...new Set(dbTransactions.map(tx => tx.txnType))].sort();

    return (
      <div className="space-y-4">
        {/* Filters */}
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="flex flex-wrap gap-4 items-center">
            <div className="flex items-center gap-2">
              <Database className="w-4 h-4 text-green-600" />
              <span className="text-sm text-green-600 font-medium">Aus Datenbank</span>
            </div>
            <div className="flex items-center gap-2">
              <label className="text-sm text-muted-foreground">Bereich:</label>
              <select
                value={filterOwnerType}
                onChange={(e) => setFilterOwnerType(e.target.value)}
                className="text-sm rounded-md border border-input bg-background px-2 py-1"
              >
                <option value="all">Alle</option>
                <option value="account">Konten</option>
                <option value="portfolio">Depots</option>
              </select>
            </div>
            <div className="flex items-center gap-2">
              <label className="text-sm text-muted-foreground">Typ:</label>
              <select
                value={filterTxnType}
                onChange={(e) => setFilterTxnType(e.target.value)}
                className="text-sm rounded-md border border-input bg-background px-2 py-1"
              >
                <option value="all">Alle</option>
                {uniqueTxnTypes.map(type => (
                  <option key={type} value={type}>{txnTypeLabels[type] || type}</option>
                ))}
              </select>
            </div>
            <div className="text-sm text-muted-foreground ml-auto">
              {filteredTransactions.length} Buchungen
            </div>
          </div>
        </div>

        {/* Transactions Table */}
        <div className="bg-card rounded-lg border border-border p-6">
          <h2 className="text-lg font-semibold mb-4">Buchungen</h2>
          {isLoading ? (
            <div className="text-muted-foreground">Lädt...</div>
          ) : filteredTransactions.length > 0 ? (
            <>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border">
                      <th className="text-left py-2 font-medium">Datum</th>
                      <th className="text-left py-2 font-medium">Typ</th>
                      <th className="text-left py-2 font-medium">Konto/Depot</th>
                      <th className="text-left py-2 font-medium">Wertpapier</th>
                      <th className="text-right py-2 font-medium">Stück</th>
                      <th className="text-right py-2 font-medium">Betrag</th>
                      <th className="text-right py-2 font-medium">Gebühren</th>
                      <th className="text-right py-2 font-medium">Steuern</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredTransactions.slice(0, displayLimit).map((tx) => {
                      const isPositive = ['BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN', 'DEPOSIT', 'DIVIDENDS', 'INTEREST', 'FEES_REFUND', 'TAX_REFUND'].includes(tx.txnType);
                      const isNegative = ['SELL', 'DELIVERY_OUTBOUND', 'TRANSFER_OUT', 'REMOVAL', 'FEES', 'TAXES', 'INTEREST_CHARGE'].includes(tx.txnType);

                      return (
                        <tr key={tx.uuid} className="border-b border-border last:border-0 hover:bg-accent/30">
                          <td className="py-2 whitespace-nowrap">{tx.date}</td>
                          <td className="py-2">
                            <span className={`inline-block px-2 py-0.5 rounded text-xs ${
                              tx.ownerType === 'portfolio' ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300' : 'bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300'
                            }`}>
                              {txnTypeLabels[tx.txnType] || tx.txnType}
                            </span>
                          </td>
                          <td className="py-2 text-muted-foreground">{tx.ownerName}</td>
                          <td className="py-2">
                            {tx.securityName ? (
                              <span className="font-medium">{tx.securityName}</span>
                            ) : (
                              <span className="text-muted-foreground">-</span>
                            )}
                          </td>
                          <td className="py-2 text-right font-mono">
                            {tx.shares !== null ? tx.shares.toLocaleString('de-DE', { maximumFractionDigits: 6 }) : '-'}
                          </td>
                          <td className={`py-2 text-right font-mono ${isPositive ? 'text-green-600' : isNegative ? 'text-red-600' : ''}`}>
                            {tx.amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} {tx.currency}
                          </td>
                          <td className="py-2 text-right font-mono text-muted-foreground">
                            {tx.fees > 0 ? tx.fees.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '-'}
                          </td>
                          <td className="py-2 text-right font-mono text-muted-foreground">
                            {tx.taxes > 0 ? tx.taxes.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '-'}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
              {filteredTransactions.length > displayLimit && (
                <div className="text-center pt-4">
                  <button
                    onClick={() => setDisplayLimit(prev => prev + 100)}
                    className="text-sm text-primary hover:underline"
                  >
                    Mehr anzeigen ({displayLimit} von {filteredTransactions.length})
                  </button>
                </div>
              )}
            </>
          ) : (
            <p className="text-muted-foreground">Keine Buchungen gefunden.</p>
          )}
        </div>
      </div>
    );
  }

  if (!portfolioFile) {
    return (
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Buchungen</h2>
        <p className="text-muted-foreground">
          Öffnen Sie eine Portfolio Performance Datei oder importieren Sie sie in die Datenbank, um Buchungen anzuzeigen.
        </p>
      </div>
    );
  }

  // Collect all transactions from all accounts
  const allTransactions: Array<AccountTransaction & { accountName: string; index: number }> = [];
  let txIndex = 0;
  for (const account of portfolioFile.accounts || []) {
    for (const tx of account.transactions || []) {
      allTransactions.push({ ...tx, accountName: account.name || 'Unbenannt', index: txIndex++ });
    }
  }

  // Sort by date descending
  allTransactions.sort((a, b) => (b.date || '').localeCompare(a.date || ''));

  return (
    <div className="bg-card rounded-lg border border-border p-6">
      <h2 className="text-lg font-semibold mb-4">Buchungen ({allTransactions.length})</h2>
      {allTransactions.length > 0 ? (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left py-2 font-medium">Datum</th>
                <th className="text-left py-2 font-medium">Typ</th>
                <th className="text-left py-2 font-medium">Konto</th>
                <th className="text-right py-2 font-medium">Betrag</th>
              </tr>
            </thead>
            <tbody>
              {allTransactions.slice(0, 50).map((tx) => (
                <tr key={tx.uuid || `tx-${tx.index}`} className="border-b border-border last:border-0">
                  <td className="py-2">{tx.date || '-'}</td>
                  <td className="py-2">{tx.transactionType || '-'}</td>
                  <td className="py-2 text-muted-foreground">{tx.accountName}</td>
                  <td className="py-2 text-right">{(tx.amount.amount / 100).toFixed(2)} {tx.amount.currency}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {allTransactions.length > 50 && (
            <div className="text-sm text-muted-foreground text-center pt-4">
              Zeige 50 von {allTransactions.length} Buchungen
            </div>
          )}
        </div>
      ) : (
        <p className="text-muted-foreground">Keine Buchungen vorhanden.</p>
      )}
    </div>
  );
}

function ReportsView() {
  return (
    <div className="bg-card rounded-lg border border-border p-6">
      <h2 className="text-lg font-semibold mb-4">Berichte</h2>
      <p className="text-muted-foreground">
        Analysieren Sie Ihre Performance mit verschiedenen Berichten und Diagrammen.
      </p>
    </div>
  );
}

function SettingsView() {
  return (
    <div className="bg-card rounded-lg border border-border p-6">
      <h2 className="text-lg font-semibold mb-4">Einstellungen</h2>
      <div className="space-y-4">
        <div>
          <label className="text-sm font-medium">Sprache</label>
          <select className="mt-1 block w-full max-w-xs rounded-md border border-input bg-background px-3 py-2">
            <option value="de">Deutsch</option>
            <option value="en">English</option>
          </select>
        </div>
        <div>
          <label className="text-sm font-medium">Design</label>
          <select className="mt-1 block w-full max-w-xs rounded-md border border-input bg-background px-3 py-2">
            <option value="light">Hell</option>
            <option value="dark">Dunkel</option>
            <option value="system">System</option>
          </select>
        </div>
        <div>
          <label className="text-sm font-medium">Basiswährung</label>
          <select className="mt-1 block w-full max-w-xs rounded-md border border-input bg-background px-3 py-2">
            <option value="EUR">EUR - Euro</option>
            <option value="USD">USD - US Dollar</option>
            <option value="CHF">CHF - Schweizer Franken</option>
          </select>
        </div>
      </div>
    </div>
  );
}

export default App;
