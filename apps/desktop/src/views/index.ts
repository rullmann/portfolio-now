// Export all views
export { DashboardView } from './Dashboard';
export { PortfolioView } from './Portfolio';
export { SecuritiesView, SecuritiesViewWithErrorBoundary } from './Securities';
export { AccountsView } from './Accounts';
export { TransactionsView } from './Transactions';
export { HoldingsView } from './Holdings';
export { AssetStatementView } from './AssetStatement';
export { WatchlistView } from './Watchlist';
export { TaxonomiesView } from './Taxonomies';
export { InvestmentPlansView } from './InvestmentPlans';
export { RebalancingView } from './Rebalancing';
export { ChartsView } from './Charts';
export { BenchmarkView } from './Benchmark';
export { ReportsView } from './Reports';
export { SettingsView } from './Settings';

// Export types
export type {
  PortfolioFile,
  AggregatedHolding,
  PortfolioData,
  Security,
  Account,
  Portfolio,
  Holding,
  GroupedHolding,
} from './types';
