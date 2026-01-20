/**
 * Dividend Calendar - Visual calendar view of dividend payments and ex-dividend dates.
 */

import { useState, useEffect, useMemo } from 'react';
import { ChevronLeft, ChevronRight, Calendar, Building2, Plus, Pencil, Trash2, Bell } from 'lucide-react';
import {
  getDividendCalendar,
  getCachedLogoData,
  fetchLogosBatch,
  getSecurities,
  getEnhancedDividendCalendar,
  getUpcomingExDividends,
  createExDividend,
  updateExDividend,
  deleteExDividend,
} from '../../lib/api';
import type { MonthCalendarData, CalendarDividend, EnhancedMonthCalendarData, DividendCalendarEvent, ExDividend, ExDividendRequest } from '../../lib/api';
import type { SecurityData } from '../../lib/types';
import { useSettingsStore, toast } from '../../store';

interface Props {
  selectedYear: number;
  onYearChange: (year: number) => void;
}

type CalendarMode = 'payments' | 'enhanced';

const MONTH_NAMES = [
  'Januar', 'Februar', 'März', 'April', 'Mai', 'Juni',
  'Juli', 'August', 'September', 'Oktober', 'November', 'Dezember'
];

const WEEKDAYS = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];

// Event type colors
const EVENT_COLORS = {
  payment: {
    bg: 'bg-green-100 dark:bg-green-900/30',
    text: 'text-green-700 dark:text-green-400',
    border: 'border-green-200 dark:border-green-800',
    dot: 'bg-green-500',
  },
  ex_dividend: {
    bg: 'bg-amber-100 dark:bg-amber-900/30',
    text: 'text-amber-700 dark:text-amber-400',
    border: 'border-amber-200 dark:border-amber-800',
    dot: 'bg-amber-500',
  },
  record_date: {
    bg: 'bg-blue-100 dark:bg-blue-900/30',
    text: 'text-blue-700 dark:text-blue-400',
    border: 'border-blue-200 dark:border-blue-800',
    dot: 'bg-blue-500',
  },
};

export function DividendCalendar({ selectedYear, onYearChange }: Props) {
  const [isLoading, setIsLoading] = useState(true);
  const [calendarData, setCalendarData] = useState<MonthCalendarData[]>([]);
  const [enhancedData, setEnhancedData] = useState<EnhancedMonthCalendarData[]>([]);
  const [upcomingExDivs, setUpcomingExDivs] = useState<ExDividend[]>([]);
  const [selectedMonth, setSelectedMonth] = useState<number>(new Date().getMonth() + 1);
  const [logos, setLogos] = useState<Map<number, string>>(new Map());
  const [calendarMode, setCalendarMode] = useState<CalendarMode>('enhanced');
  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [showExDivForm, setShowExDivForm] = useState(false);
  const [editingExDiv, setEditingExDiv] = useState<ExDividend | null>(null);

  const { brandfetchApiKey } = useSettingsStore();

  // Load calendar data
  useEffect(() => {
    async function loadData() {
      setIsLoading(true);
      try {
        // Load all data in parallel
        const [paymentData, enhancedCalData, upcomingData, allSecurities] = await Promise.all([
          getDividendCalendar(selectedYear),
          getEnhancedDividendCalendar(selectedYear),
          getUpcomingExDividends(30),
          getSecurities(),
        ]);

        setCalendarData(paymentData);
        setEnhancedData(enhancedCalData);
        setUpcomingExDivs(upcomingData);
        setSecurities(allSecurities);

        // Collect security IDs from all data sources
        const securityIds = new Set<number>();
        paymentData.forEach(m => m.dividends.forEach(d => securityIds.add(d.securityId)));
        enhancedCalData.forEach(m => m.events.forEach(e => securityIds.add(e.securityId)));
        upcomingData.forEach(d => securityIds.add(d.securityId));

        if (securityIds.size > 0) {
          const secMap = new Map<number, SecurityData>();
          allSecurities.forEach(s => secMap.set(s.id, s));

          const toFetch = Array.from(securityIds)
            .map(id => secMap.get(id))
            .filter((s): s is SecurityData => !!s)
            .map(s => ({ id: s.id, ticker: s.ticker || undefined, name: s.name }));

          if (toFetch.length > 0) {
            const results = await fetchLogosBatch(brandfetchApiKey || '', toFetch);
            const newLogos = new Map<number, string>();

            for (const result of results) {
              if (result.domain) {
                const cached = await getCachedLogoData(result.domain);
                if (cached) {
                  newLogos.set(result.securityId, cached);
                } else if (result.logoUrl) {
                  newLogos.set(result.securityId, result.logoUrl);
                }
              }
            }
            setLogos(newLogos);
          }
        }
      } catch (err) {
        console.error('Failed to load calendar data:', err);
      } finally {
        setIsLoading(false);
      }
    }
    loadData();
  }, [selectedYear, brandfetchApiKey]);

  // Get current month data (payment mode)
  const monthData = useMemo(() => {
    return calendarData.find(m => m.month === selectedMonth);
  }, [calendarData, selectedMonth]);

  // Get current month enhanced data
  const enhancedMonthData = useMemo(() => {
    return enhancedData.find(m => m.month === selectedMonth);
  }, [enhancedData, selectedMonth]);

  // Group dividends by day (payment mode)
  const dividendsByDay = useMemo(() => {
    const map = new Map<number, CalendarDividend[]>();
    if (monthData) {
      monthData.dividends.forEach(d => {
        const day = parseInt(d.date.split('-')[2], 10);
        const existing = map.get(day);
        if (existing) {
          existing.push(d);
        } else {
          map.set(day, [d]);
        }
      });
    }
    return map;
  }, [monthData]);

  // Group events by day (enhanced mode)
  const eventsByDay = useMemo(() => {
    const map = new Map<number, DividendCalendarEvent[]>();
    if (enhancedMonthData) {
      enhancedMonthData.events.forEach(e => {
        const day = parseInt(e.date.split('-')[2], 10);
        const existing = map.get(day);
        if (existing) {
          existing.push(e);
        } else {
          map.set(day, [e]);
        }
      });
    }
    return map;
  }, [enhancedMonthData]);

  // Calculate calendar grid
  const calendarGrid = useMemo(() => {
    const firstDay = new Date(selectedYear, selectedMonth - 1, 1);
    const lastDay = new Date(selectedYear, selectedMonth, 0);
    const daysInMonth = lastDay.getDate();

    // Adjust for Monday start (0 = Mon, 6 = Sun)
    let startDay = firstDay.getDay() - 1;
    if (startDay < 0) startDay = 6;

    const weeks: (number | null)[][] = [];
    let currentWeek: (number | null)[] = [];

    // Fill empty days at start
    for (let i = 0; i < startDay; i++) {
      currentWeek.push(null);
    }

    // Fill days
    for (let day = 1; day <= daysInMonth; day++) {
      currentWeek.push(day);
      if (currentWeek.length === 7) {
        weeks.push(currentWeek);
        currentWeek = [];
      }
    }

    // Fill remaining days
    if (currentWeek.length > 0) {
      while (currentWeek.length < 7) {
        currentWeek.push(null);
      }
      weeks.push(currentWeek);
    }

    return weeks;
  }, [selectedYear, selectedMonth]);

  const formatCurrency = (amount: number, currency: string = 'EUR') => {
    return `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
  };

  const prevMonth = () => {
    if (selectedMonth === 1) {
      onYearChange(selectedYear - 1);
      setSelectedMonth(12);
    } else {
      setSelectedMonth(selectedMonth - 1);
    }
  };

  const nextMonth = () => {
    if (selectedMonth === 12) {
      onYearChange(selectedYear + 1);
      setSelectedMonth(1);
    } else {
      setSelectedMonth(selectedMonth + 1);
    }
  };

  const SecurityLogo = ({ securityId, size = 20 }: { securityId: number; size?: number }) => {
    const logoUrl = logos.get(securityId);
    if (logoUrl) {
      return (
        <img
          src={logoUrl}
          alt=""
          className="rounded-sm object-contain bg-white flex-shrink-0"
          style={{ width: size, height: size }}
        />
      );
    }
    return (
      <div
        className="rounded-sm bg-muted flex items-center justify-center flex-shrink-0"
        style={{ width: size, height: size }}
      >
        <Building2 size={size * 0.6} className="text-muted-foreground" />
      </div>
    );
  };

  // Get event type label in German
  const getEventTypeLabel = (type: string): string => {
    switch (type) {
      case 'ex_dividend': return 'Ex-Dividende';
      case 'record_date': return 'Record Date';
      case 'payment': return 'Zahlung';
      default: return type;
    }
  };

  // Reload data after ex-dividend changes
  const reloadData = async () => {
    try {
      const [enhancedCalData, upcomingData] = await Promise.all([
        getEnhancedDividendCalendar(selectedYear),
        getUpcomingExDividends(30),
      ]);
      setEnhancedData(enhancedCalData);
      setUpcomingExDivs(upcomingData);
    } catch (err) {
      console.error('Failed to reload data:', err);
    }
  };

  // Handle ex-dividend delete
  const handleDeleteExDiv = async (id: number) => {
    try {
      await deleteExDividend(id);
      toast.success('Ex-Dividende gelöscht');
      await reloadData();
    } catch (err) {
      toast.error('Fehler beim Löschen');
      console.error(err);
    }
  };

  return (
    <div className="space-y-4">
      {/* Mode Toggle & Actions */}
      <div className="flex items-center justify-between">
        <div className="flex gap-1 p-1 bg-muted rounded-lg">
          <button
            onClick={() => setCalendarMode('enhanced')}
            className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
              calendarMode === 'enhanced'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            Alle Termine
          </button>
          <button
            onClick={() => setCalendarMode('payments')}
            className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
              calendarMode === 'payments'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            Nur Zahlungen
          </button>
        </div>

        <button
          onClick={() => {
            setEditingExDiv(null);
            setShowExDivForm(true);
          }}
          className="flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-md text-sm hover:bg-primary/90 transition-colors"
        >
          <Plus size={16} />
          Ex-Dividende eintragen
        </button>
      </div>

      {/* Legend */}
      {calendarMode === 'enhanced' && (
        <div className="flex items-center gap-4 text-sm">
          <div className="flex items-center gap-2">
            <div className={`w-3 h-3 rounded-full ${EVENT_COLORS.ex_dividend.dot}`} />
            <span className="text-muted-foreground">Ex-Dividende</span>
          </div>
          <div className="flex items-center gap-2">
            <div className={`w-3 h-3 rounded-full ${EVENT_COLORS.record_date.dot}`} />
            <span className="text-muted-foreground">Record Date</span>
          </div>
          <div className="flex items-center gap-2">
            <div className={`w-3 h-3 rounded-full ${EVENT_COLORS.payment.dot}`} />
            <span className="text-muted-foreground">Zahlung</span>
          </div>
        </div>
      )}

      {/* Month Navigation */}
      <div className="flex items-center justify-between">
        <button
          onClick={prevMonth}
          className="p-2 hover:bg-muted rounded-md transition-colors"
        >
          <ChevronLeft size={20} />
        </button>
        <h3 className="text-lg font-semibold">
          {MONTH_NAMES[selectedMonth - 1]} {selectedYear}
        </h3>
        <button
          onClick={nextMonth}
          className="p-2 hover:bg-muted rounded-md transition-colors"
        >
          <ChevronRight size={20} />
        </button>
      </div>

      {/* Monthly Summary */}
      {calendarMode === 'payments' && monthData && (
        <div className="bg-green-50 dark:bg-green-900/20 rounded-lg p-4 text-center">
          <div className="text-sm text-muted-foreground">Dividenden im {MONTH_NAMES[selectedMonth - 1]}</div>
          <div className="text-2xl font-bold text-green-600">
            {formatCurrency(monthData.totalAmount, monthData.currency)}
          </div>
          <div className="text-xs text-muted-foreground mt-1">
            {monthData.dividends.length} Zahlungen
          </div>
        </div>
      )}

      {calendarMode === 'enhanced' && enhancedMonthData && (
        <div className="grid grid-cols-3 gap-3">
          <div className="bg-amber-50 dark:bg-amber-900/20 rounded-lg p-3 text-center">
            <div className="text-xs text-muted-foreground">Ex-Dividenden</div>
            <div className="text-xl font-bold text-amber-600">{enhancedMonthData.totalExDividends}</div>
          </div>
          <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-3 text-center">
            <div className="text-xs text-muted-foreground">Record Dates</div>
            <div className="text-xl font-bold text-blue-600">
              {enhancedMonthData.events.filter(e => e.eventType === 'record_date').length}
            </div>
          </div>
          <div className="bg-green-50 dark:bg-green-900/20 rounded-lg p-3 text-center">
            <div className="text-xs text-muted-foreground">Zahlungen</div>
            <div className="text-xl font-bold text-green-600">{enhancedMonthData.totalPayments}</div>
          </div>
        </div>
      )}

      {/* Calendar Grid */}
      <div className="bg-card rounded-lg border border-border overflow-hidden">
        {/* Weekday Header */}
        <div className="grid grid-cols-7 bg-muted/50">
          {WEEKDAYS.map(day => (
            <div key={day} className="py-2 text-center text-sm font-medium text-muted-foreground">
              {day}
            </div>
          ))}
        </div>

        {/* Calendar Days */}
        {isLoading ? (
          <div className="h-64 flex items-center justify-center text-muted-foreground">
            Lade Kalender...
          </div>
        ) : calendarMode === 'payments' ? (
          /* Payment-only mode */
          <div className="divide-y divide-border">
            {calendarGrid.map((week, weekIdx) => (
              <div key={weekIdx} className="grid grid-cols-7 divide-x divide-border">
                {week.map((day, dayIdx) => {
                  const dividends = day ? dividendsByDay.get(day) : undefined;
                  const hasPayment = dividends && dividends.length > 0;
                  const isToday = day === new Date().getDate()
                    && selectedMonth === new Date().getMonth() + 1
                    && selectedYear === new Date().getFullYear();

                  return (
                    <div
                      key={dayIdx}
                      className={`min-h-[80px] p-1 ${
                        day ? 'bg-background' : 'bg-muted/30'
                      } ${isToday ? 'ring-2 ring-primary ring-inset' : ''}`}
                    >
                      {day && (
                        <>
                          <div className={`text-xs font-medium mb-1 ${
                            hasPayment ? 'text-green-600' : 'text-muted-foreground'
                          }`}>
                            {day}
                          </div>
                          {hasPayment && dividends && (
                            <div className="space-y-1">
                              {dividends.slice(0, 3).map((d, idx) => (
                                <div
                                  key={idx}
                                  className="flex items-center gap-1 bg-green-100 dark:bg-green-900/30 rounded px-1 py-0.5 text-xs"
                                  title={`${d.securityName}: ${formatCurrency(d.amount, d.currency)}`}
                                >
                                  <SecurityLogo securityId={d.securityId} size={14} />
                                  <span className="truncate flex-1 text-green-700 dark:text-green-400">
                                    {formatCurrency(d.amount, d.currency)}
                                  </span>
                                </div>
                              ))}
                              {dividends.length > 3 && (
                                <div className="text-xs text-muted-foreground text-center">
                                  +{dividends.length - 3} weitere
                                </div>
                              )}
                            </div>
                          )}
                        </>
                      )}
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        ) : (
          /* Enhanced mode with all event types */
          <div className="divide-y divide-border">
            {calendarGrid.map((week, weekIdx) => (
              <div key={weekIdx} className="grid grid-cols-7 divide-x divide-border">
                {week.map((day, dayIdx) => {
                  const events = day ? eventsByDay.get(day) : undefined;
                  const hasEvents = events && events.length > 0;
                  const isToday = day === new Date().getDate()
                    && selectedMonth === new Date().getMonth() + 1
                    && selectedYear === new Date().getFullYear();

                  // Determine day number color based on events
                  let dayColor = 'text-muted-foreground';
                  if (hasEvents && events) {
                    const hasExDiv = events.some(e => e.eventType === 'ex_dividend');
                    const hasPaymentEvent = events.some(e => e.eventType === 'payment');
                    if (hasExDiv) dayColor = 'text-amber-600';
                    else if (hasPaymentEvent) dayColor = 'text-green-600';
                  }

                  return (
                    <div
                      key={dayIdx}
                      className={`min-h-[80px] p-1 ${
                        day ? 'bg-background' : 'bg-muted/30'
                      } ${isToday ? 'ring-2 ring-primary ring-inset' : ''}`}
                    >
                      {day && (
                        <>
                          <div className={`text-xs font-medium mb-1 ${dayColor}`}>
                            {day}
                          </div>
                          {hasEvents && events && (
                            <div className="space-y-1">
                              {events.slice(0, 3).map((e, idx) => {
                                const colors = EVENT_COLORS[e.eventType as keyof typeof EVENT_COLORS] || EVENT_COLORS.payment;
                                return (
                                  <div
                                    key={idx}
                                    className={`flex items-center gap-1 ${colors.bg} rounded px-1 py-0.5 text-xs`}
                                    title={`${getEventTypeLabel(e.eventType)}: ${e.securityName}${e.amount ? ` - ${formatCurrency(e.amount, e.currency || 'EUR')}` : ''}`}
                                  >
                                    <div className={`w-1.5 h-1.5 rounded-full ${colors.dot} flex-shrink-0`} />
                                    <SecurityLogo securityId={e.securityId} size={14} />
                                    <span className={`truncate flex-1 ${colors.text}`}>
                                      {e.securityName.split(' ')[0]}
                                    </span>
                                  </div>
                                );
                              })}
                              {events.length > 3 && (
                                <div className="text-xs text-muted-foreground text-center">
                                  +{events.length - 3} weitere
                                </div>
                              )}
                            </div>
                          )}
                        </>
                      )}
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Upcoming Ex-Dividends Alert */}
      {upcomingExDivs.length > 0 && (
        <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-3">
            <Bell size={18} className="text-amber-600" />
            <h4 className="font-medium text-amber-800 dark:text-amber-200">
              Anstehende Ex-Dividenden (nächste 30 Tage)
            </h4>
          </div>
          <div className="space-y-2">
            {upcomingExDivs.slice(0, 5).map((exDiv) => (
              <div
                key={exDiv.id}
                className="flex items-center justify-between bg-white dark:bg-amber-950/30 rounded-md p-2"
              >
                <div className="flex items-center gap-3">
                  <SecurityLogo securityId={exDiv.securityId} size={24} />
                  <div>
                    <div className="font-medium text-sm">{exDiv.securityName}</div>
                    <div className="text-xs text-muted-foreground">
                      Ex-Div: {exDiv.exDate}
                      {exDiv.payDate && ` → Zahlung: ${exDiv.payDate}`}
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {exDiv.amount && (
                    <span className="text-sm font-medium text-amber-700 dark:text-amber-400">
                      {formatCurrency(exDiv.amount, exDiv.currency || 'EUR')}
                    </span>
                  )}
                  <button
                    onClick={() => {
                      setEditingExDiv(exDiv);
                      setShowExDivForm(true);
                    }}
                    className="p-1 hover:bg-amber-100 dark:hover:bg-amber-900/50 rounded"
                    title="Bearbeiten"
                  >
                    <Pencil size={14} className="text-muted-foreground" />
                  </button>
                  <button
                    onClick={() => handleDeleteExDiv(exDiv.id)}
                    className="p-1 hover:bg-red-100 dark:hover:bg-red-900/50 rounded"
                    title="Löschen"
                  >
                    <Trash2 size={14} className="text-red-500" />
                  </button>
                </div>
              </div>
            ))}
            {upcomingExDivs.length > 5 && (
              <div className="text-xs text-muted-foreground text-center pt-1">
                +{upcomingExDivs.length - 5} weitere Ex-Dividenden
              </div>
            )}
          </div>
        </div>
      )}

      {/* Event List for Selected Month (Enhanced Mode) */}
      {calendarMode === 'enhanced' && enhancedMonthData && enhancedMonthData.events.length > 0 && (
        <div className="bg-card rounded-lg border border-border">
          <div className="p-3 border-b border-border">
            <h4 className="font-medium">Alle Termine im {MONTH_NAMES[selectedMonth - 1]}</h4>
          </div>
          <div className="divide-y divide-border max-h-64 overflow-y-auto">
            {enhancedMonthData.events
              .sort((a, b) => a.date.localeCompare(b.date))
              .map((e, idx) => {
                const colors = EVENT_COLORS[e.eventType as keyof typeof EVENT_COLORS] || EVENT_COLORS.payment;
                return (
                  <div key={idx} className="flex items-center justify-between p-3 hover:bg-muted/30">
                    <div className="flex items-center gap-3">
                      <div className={`w-2 h-2 rounded-full ${colors.dot}`} />
                      <SecurityLogo securityId={e.securityId} size={28} />
                      <div>
                        <div className="font-medium">{e.securityName}</div>
                        <div className="text-xs text-muted-foreground">
                          {e.date} · {getEventTypeLabel(e.eventType)}
                          {!e.isConfirmed && ' (geschätzt)'}
                        </div>
                      </div>
                    </div>
                    <div className="text-right">
                      {e.amount && (
                        <div className={`font-medium ${colors.text}`}>
                          {formatCurrency(e.amount, e.currency || 'EUR')}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
          </div>
        </div>
      )}

      {/* Payment List for Selected Month (Payment Mode) */}
      {calendarMode === 'payments' && monthData && monthData.dividends.length > 0 && (
        <div className="bg-card rounded-lg border border-border">
          <div className="p-3 border-b border-border">
            <h4 className="font-medium">Alle Zahlungen im {MONTH_NAMES[selectedMonth - 1]}</h4>
          </div>
          <div className="divide-y divide-border max-h-64 overflow-y-auto">
            {monthData.dividends.map((d, idx) => (
              <div key={idx} className="flex items-center justify-between p-3 hover:bg-muted/30">
                <div className="flex items-center gap-3">
                  <SecurityLogo securityId={d.securityId} size={28} />
                  <div>
                    <div className="font-medium">{d.securityName}</div>
                    <div className="text-xs text-muted-foreground">{d.date}</div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="font-medium text-green-600">
                    {formatCurrency(d.amount, d.currency)}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Empty State */}
      {!isLoading && calendarMode === 'payments' && (!monthData || monthData.dividends.length === 0) && (
        <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
          <Calendar className="w-12 h-12 mx-auto mb-3 opacity-50" />
          <p>Keine Dividenden im {MONTH_NAMES[selectedMonth - 1]} {selectedYear}.</p>
        </div>
      )}

      {!isLoading && calendarMode === 'enhanced' && (!enhancedMonthData || enhancedMonthData.events.length === 0) && (
        <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
          <Calendar className="w-12 h-12 mx-auto mb-3 opacity-50" />
          <p>Keine Termine im {MONTH_NAMES[selectedMonth - 1]} {selectedYear}.</p>
          <p className="text-sm mt-2">
            Klicke auf "Ex-Dividende eintragen" um Ex-Dividenden-Termine hinzuzufügen.
          </p>
        </div>
      )}

      {/* Ex-Dividend Form Modal */}
      {showExDivForm && (
        <ExDividendFormModal
          securities={securities}
          editingExDiv={editingExDiv}
          onClose={() => {
            setShowExDivForm(false);
            setEditingExDiv(null);
          }}
          onSave={async (request) => {
            try {
              if (editingExDiv) {
                await updateExDividend(editingExDiv.id, request);
                toast.success('Ex-Dividende aktualisiert');
              } else {
                await createExDividend(request);
                toast.success('Ex-Dividende erstellt');
              }
              await reloadData();
              setShowExDivForm(false);
              setEditingExDiv(null);
            } catch (err) {
              toast.error(editingExDiv ? 'Fehler beim Aktualisieren' : 'Fehler beim Erstellen');
              console.error(err);
            }
          }}
        />
      )}
    </div>
  );
}

// Ex-Dividend Form Modal Component
interface ExDividendFormModalProps {
  securities: SecurityData[];
  editingExDiv: ExDividend | null;
  onClose: () => void;
  onSave: (request: ExDividendRequest) => Promise<void>;
}

function ExDividendFormModal({ securities, editingExDiv, onClose, onSave }: ExDividendFormModalProps) {
  const [securityId, setSecurityId] = useState<number>(editingExDiv?.securityId || 0);
  const [exDate, setExDate] = useState(editingExDiv?.exDate || '');
  const [recordDate, setRecordDate] = useState(editingExDiv?.recordDate || '');
  const [payDate, setPayDate] = useState(editingExDiv?.payDate || '');
  const [amount, setAmount] = useState(editingExDiv?.amount?.toString() || '');
  const [currency, setCurrency] = useState(editingExDiv?.currency || 'EUR');
  const [frequency, setFrequency] = useState(editingExDiv?.frequency || '');
  const [note, setNote] = useState(editingExDiv?.note || '');
  const [isConfirmed, setIsConfirmed] = useState(editingExDiv?.isConfirmed ?? true);
  const [isSaving, setIsSaving] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!securityId || !exDate) return;

    setIsSaving(true);
    try {
      await onSave({
        securityId,
        exDate,
        recordDate: recordDate || undefined,
        payDate: payDate || undefined,
        amount: amount ? parseFloat(amount) : undefined,
        currency: currency || undefined,
        frequency: frequency || undefined,
        isConfirmed,
        note: note || undefined,
      });
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background rounded-lg shadow-xl w-full max-w-md mx-4">
        <div className="p-4 border-b border-border">
          <h3 className="text-lg font-semibold">
            {editingExDiv ? 'Ex-Dividende bearbeiten' : 'Ex-Dividende eintragen'}
          </h3>
        </div>

        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          {/* Security Select */}
          <div>
            <label className="block text-sm font-medium mb-1">Wertpapier *</label>
            <select
              value={securityId}
              onChange={(e) => setSecurityId(Number(e.target.value))}
              className="w-full px-3 py-2 border border-border rounded-md bg-background"
              required
            >
              <option value={0}>Wertpapier auswählen...</option>
              {securities.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name} {s.isin ? `(${s.isin})` : ''}
                </option>
              ))}
            </select>
          </div>

          {/* Dates */}
          <div className="grid grid-cols-3 gap-3">
            <div>
              <label className="block text-sm font-medium mb-1">Ex-Datum *</label>
              <input
                type="date"
                value={exDate}
                onChange={(e) => setExDate(e.target.value)}
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
                required
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Record Date</label>
              <input
                type="date"
                value={recordDate}
                onChange={(e) => setRecordDate(e.target.value)}
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Zahldatum</label>
              <input
                type="date"
                value={payDate}
                onChange={(e) => setPayDate(e.target.value)}
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
              />
            </div>
          </div>

          {/* Amount & Currency */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium mb-1">Betrag pro Aktie</label>
              <input
                type="number"
                step="0.0001"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder="0.00"
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Währung</label>
              <select
                value={currency}
                onChange={(e) => setCurrency(e.target.value)}
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
              >
                <option value="EUR">EUR</option>
                <option value="USD">USD</option>
                <option value="GBP">GBP</option>
                <option value="CHF">CHF</option>
              </select>
            </div>
          </div>

          {/* Frequency */}
          <div>
            <label className="block text-sm font-medium mb-1">Frequenz</label>
            <select
              value={frequency}
              onChange={(e) => setFrequency(e.target.value)}
              className="w-full px-3 py-2 border border-border rounded-md bg-background"
            >
              <option value="">Nicht angegeben</option>
              <option value="MONTHLY">Monatlich</option>
              <option value="QUARTERLY">Vierteljährlich</option>
              <option value="SEMI_ANNUAL">Halbjährlich</option>
              <option value="ANNUAL">Jährlich</option>
            </select>
          </div>

          {/* Confirmed checkbox */}
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="isConfirmed"
              checked={isConfirmed}
              onChange={(e) => setIsConfirmed(e.target.checked)}
              className="rounded border-border"
            />
            <label htmlFor="isConfirmed" className="text-sm">
              Bestätigt (nicht geschätzt)
            </label>
          </div>

          {/* Note */}
          <div>
            <label className="block text-sm font-medium mb-1">Notiz</label>
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              rows={2}
              className="w-full px-3 py-2 border border-border rounded-md bg-background resize-none"
              placeholder="Optionale Notiz..."
            />
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 border border-border rounded-md hover:bg-muted transition-colors"
            >
              Abbrechen
            </button>
            <button
              type="submit"
              disabled={isSaving || !securityId || !exDate}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              {isSaving ? 'Speichern...' : 'Speichern'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
