import { XMLParser } from 'fast-xml-parser';
import type { PPClient, PPSecurity, PPAccount, PPPortfolio, PPTaxonomy } from './types';

const parserOptions = {
  ignoreAttributes: false,
  attributeNamePrefix: '@_',
  textNodeName: '#text',
  parseAttributeValue: true,
  trimValues: true,
};

export function parsePortfolioFile(xmlContent: string): PPClient {
  const parser = new XMLParser(parserOptions);
  const parsed = parser.parse(xmlContent);

  const client = parsed.client;

  if (!client) {
    throw new Error('Invalid Portfolio Performance file: missing client element');
  }

  return {
    version: client.version || 0,
    baseCurrency: client.baseCurrency || 'EUR',
    securities: parseSecurities(client.securities),
    watchlists: [],
    accounts: parseAccounts(client.accounts),
    portfolios: parsePortfolios(client.portfolios),
    plans: [],
    taxonomies: parseTaxonomies(client.taxonomies),
    dashboards: [],
    properties: {},
    settings: {},
  };
}

function parseSecurities(securities: any): PPSecurity[] {
  if (!securities?.security) return [];

  const items = Array.isArray(securities.security)
    ? securities.security
    : [securities.security];

  return items.map((s: any) => ({
    uuid: s.uuid || '',
    name: s.name || '',
    currencyCode: s.currencyCode || 'EUR',
    isin: s.isin,
    wkn: s.wkn,
    tickerSymbol: s.tickerSymbol,
    note: s.note,
    isRetired: s.isRetired === true || s.isRetired === 'true',
    prices: parsePrices(s.prices),
    feed: s.feed,
    feedURL: s.feedURL,
  }));
}

function parsePrices(prices: any): { date: string; value: number }[] {
  if (!prices?.price) return [];

  const items = Array.isArray(prices.price) ? prices.price : [prices.price];

  return items.map((p: any) => ({
    date: p.t || p['@_t'] || '',
    value: parseInt(p.v || p['@_v'] || '0', 10),
  }));
}

function parseAccounts(accounts: any): PPAccount[] {
  if (!accounts?.account) return [];

  const items = Array.isArray(accounts.account)
    ? accounts.account
    : [accounts.account];

  return items.map((a: any) => ({
    uuid: a.uuid || '',
    name: a.name || '',
    currencyCode: a.currencyCode || 'EUR',
    note: a.note,
    isRetired: a.isRetired === true || a.isRetired === 'true',
    transactions: parseAccountTransactions(a.transactions),
  }));
}

function parseAccountTransactions(transactions: any): any[] {
  if (!transactions?.['account-transaction']) return [];

  const items = Array.isArray(transactions['account-transaction'])
    ? transactions['account-transaction']
    : [transactions['account-transaction']];

  return items.map((t: any) => ({
    uuid: t.uuid || '',
    date: t.date || '',
    type: t.type || '',
    amount: parseInt(t.amount || '0', 10),
    currencyCode: t.currencyCode || 'EUR',
    note: t.note,
  }));
}

function parsePortfolios(portfolios: any): PPPortfolio[] {
  if (!portfolios?.portfolio) return [];

  const items = Array.isArray(portfolios.portfolio)
    ? portfolios.portfolio
    : [portfolios.portfolio];

  return items.map((p: any) => ({
    uuid: p.uuid || '',
    name: p.name || '',
    referenceAccount: p.referenceAccount || '',
    transactions: parsePortfolioTransactions(p.transactions),
  }));
}

function parsePortfolioTransactions(transactions: any): any[] {
  if (!transactions?.['portfolio-transaction']) return [];

  const items = Array.isArray(transactions['portfolio-transaction'])
    ? transactions['portfolio-transaction']
    : [transactions['portfolio-transaction']];

  return items.map((t: any) => ({
    uuid: t.uuid || '',
    date: t.date || '',
    type: t.type || '',
    shares: parseInt(t.shares || '0', 10),
    amount: parseInt(t.amount || '0', 10),
    currencyCode: t.currencyCode || 'EUR',
    fees: parseInt(t.fees || '0', 10),
    taxes: parseInt(t.taxes || '0', 10),
    security: t.security || '',
    note: t.note,
  }));
}

function parseTaxonomies(taxonomies: any): PPTaxonomy[] {
  if (!taxonomies?.taxonomy) return [];

  const items = Array.isArray(taxonomies.taxonomy)
    ? taxonomies.taxonomy
    : [taxonomies.taxonomy];

  return items.map((t: any) => ({
    id: t.id || '',
    name: t.name || '',
    root: t.root ? parseClassification(t.root) : undefined,
  }));
}

function parseClassification(classification: any): any {
  return {
    id: classification.id || '',
    name: classification.name || '',
    color: classification.color,
    children: classification.children?.classification
      ? (Array.isArray(classification.children.classification)
          ? classification.children.classification
          : [classification.children.classification]
        ).map(parseClassification)
      : [],
  };
}
