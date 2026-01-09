import { XMLBuilder } from 'fast-xml-parser';
import type { PPClient } from './types';

const builderOptions = {
  ignoreAttributes: false,
  attributeNamePrefix: '@_',
  format: true,
  indentBy: '  ',
  suppressEmptyNode: false,
};

export function writePortfolioFile(client: PPClient): string {
  const builder = new XMLBuilder(builderOptions);

  const xmlObj = {
    '?xml': {
      '@_version': '1.0',
      '@_encoding': 'UTF-8',
    },
    client: {
      version: client.version,
      securities: {
        security: client.securities.map((s) => ({
          uuid: s.uuid,
          name: s.name,
          currencyCode: s.currencyCode,
          ...(s.isin && { isin: s.isin }),
          ...(s.wkn && { wkn: s.wkn }),
          ...(s.tickerSymbol && { tickerSymbol: s.tickerSymbol }),
          ...(s.note && { note: s.note }),
          isRetired: s.isRetired,
          prices: {
            price: s.prices.map((p) => ({
              t: p.date,
              v: p.value,
            })),
          },
          ...(s.feed && { feed: s.feed }),
          ...(s.feedURL && { feedURL: s.feedURL }),
        })),
      },
      accounts: {
        account: client.accounts.map((a) => ({
          uuid: a.uuid,
          name: a.name,
          currencyCode: a.currencyCode,
          ...(a.note && { note: a.note }),
          isRetired: a.isRetired,
          transactions: {
            'account-transaction': a.transactions.map((t) => ({
              uuid: t.uuid,
              date: t.date,
              type: t.type,
              amount: t.amount,
              currencyCode: t.currencyCode,
              ...(t.note && { note: t.note }),
            })),
          },
        })),
      },
      portfolios: {
        portfolio: client.portfolios.map((p) => ({
          uuid: p.uuid,
          name: p.name,
          referenceAccount: p.referenceAccount,
          transactions: {
            'portfolio-transaction': p.transactions.map((t) => ({
              uuid: t.uuid,
              date: t.date,
              type: t.type,
              shares: t.shares,
              amount: t.amount,
              currencyCode: t.currencyCode,
              fees: t.fees,
              taxes: t.taxes,
              security: t.security,
              ...(t.note && { note: t.note }),
            })),
          },
        })),
      },
      taxonomies: {
        taxonomy: client.taxonomies.map((t) => ({
          id: t.id,
          name: t.name,
          ...(t.root && { root: buildClassification(t.root) }),
        })),
      },
    },
  };

  return builder.build(xmlObj);
}

function buildClassification(classification: any): any {
  return {
    id: classification.id,
    name: classification.name,
    ...(classification.color && { color: classification.color }),
    ...(classification.children?.length > 0 && {
      children: {
        classification: classification.children.map(buildClassification),
      },
    }),
  };
}
