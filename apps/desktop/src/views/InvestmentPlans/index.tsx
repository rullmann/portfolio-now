/**
 * Investment Plans view for managing savings plans.
 */

import { useState, useEffect } from 'react';
import { CalendarClock, Plus, Play, Pause, Trash2, RefreshCw, AlertCircle, Edit2, CheckCircle2 } from 'lucide-react';
import { getInvestmentPlans, getPlansDueForExecution, deleteInvestmentPlan, updateInvestmentPlan, executeInvestmentPlan } from '../../lib/api';
import type { InvestmentPlanData } from '../../lib/types';
import { InvestmentPlanFormModal } from '../../components/modals';
import { toast } from '../../store';

const intervalLabels: Record<string, string> = {
  WEEKLY: 'Wöchentlich',
  BIWEEKLY: 'Alle 2 Wochen',
  MONTHLY: 'Monatlich',
  QUARTERLY: 'Quartalsweise',
  YEARLY: 'Jährlich',
};

export function InvestmentPlansView() {
  const [plans, setPlans] = useState<InvestmentPlanData[]>([]);
  const [duePlans, setDuePlans] = useState<InvestmentPlanData[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [executingPlanId, setExecutingPlanId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Modal state
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingPlan, setEditingPlan] = useState<InvestmentPlanData | null>(null);

  const loadPlans = async () => {
    try {
      setIsLoading(true);
      setError(null);
      const today = new Date().toISOString().split('T')[0];
      const [allPlans, due] = await Promise.all([
        getInvestmentPlans(),
        getPlansDueForExecution(today),
      ]);
      setPlans(allPlans);
      setDuePlans(due);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadPlans();
  }, []);

  const handleToggleActive = async (plan: InvestmentPlanData) => {
    try {
      await updateInvestmentPlan(plan.id, { isActive: !plan.isActive });
      toast.success(plan.isActive ? 'Sparplan pausiert' : 'Sparplan aktiviert');
      await loadPlans();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleDelete = async (planId: number) => {
    if (!confirm('Sparplan wirklich löschen?')) return;
    try {
      await deleteInvestmentPlan(planId);
      toast.success('Sparplan gelöscht');
      await loadPlans();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleExecute = async (plan: InvestmentPlanData) => {
    try {
      setExecutingPlanId(plan.id);
      const result = await executeInvestmentPlan(plan.id, new Date().toISOString().split('T')[0]);
      toast.success(`Sparplan ausgeführt: ${result.shares.toFixed(4)} Stück zu ${(result.price / 100).toFixed(2)} ${plan.currency}`);
      await loadPlans();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      toast.error('Ausführung fehlgeschlagen');
    } finally {
      setExecutingPlanId(null);
    }
  };

  const handleEdit = (plan: InvestmentPlanData) => {
    setEditingPlan(plan);
    setIsModalOpen(true);
  };

  const handleCreate = () => {
    setEditingPlan(null);
    setIsModalOpen(true);
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setEditingPlan(null);
  };

  const handleModalSuccess = () => {
    loadPlans();
    toast.success(editingPlan ? 'Sparplan aktualisiert' : 'Sparplan erstellt');
  };

  const formatDate = (dateStr: string | undefined) => {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleDateString('de-DE');
  };

  const formatCurrency = (amount: number, currency: string) => {
    return `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
  };

  const activePlans = plans.filter((p) => p.isActive);
  const monthlyTotal = activePlans.reduce((sum, p) => {
    // Convert all intervals to monthly equivalent
    let multiplier = 1;
    switch (p.interval) {
      case 'WEEKLY': multiplier = 4.33; break;
      case 'BIWEEKLY': multiplier = 2.17; break;
      case 'MONTHLY': multiplier = 1; break;
      case 'QUARTERLY': multiplier = 0.33; break;
      case 'YEARLY': multiplier = 0.083; break;
    }
    return sum + (p.amount * multiplier);
  }, 0);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <CalendarClock className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Sparpläne</h1>
        </div>
        <div className="flex gap-2">
          <button
            onClick={loadPlans}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
          <button
            onClick={handleCreate}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
          >
            <Plus size={16} />
            Neuer Sparplan
          </button>
        </div>
      </div>

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          {error}
        </div>
      )}

      {/* Due plans alert */}
      {duePlans.length > 0 && (
        <div className="p-4 bg-amber-500/10 border border-amber-500/20 rounded-lg">
          <div className="flex items-center gap-2 text-amber-600 font-medium mb-2">
            <AlertCircle size={18} />
            {duePlans.length} Sparplan{duePlans.length !== 1 ? 'e' : ''} zur Ausführung fällig
          </div>
          <div className="space-y-2">
            {duePlans.map((plan) => (
              <div
                key={plan.id}
                className="flex items-center justify-between bg-background/50 p-2 rounded-md"
              >
                <div>
                  <span className="font-medium">{plan.securityName}</span>
                  <span className="text-muted-foreground mx-2">·</span>
                  <span>{formatCurrency(plan.amount, plan.currency)}</span>
                </div>
                <button
                  onClick={() => handleExecute(plan)}
                  disabled={executingPlanId === plan.id}
                  className="flex items-center gap-1 px-3 py-1 text-sm bg-amber-600 text-white rounded-md hover:bg-amber-700 disabled:opacity-50"
                >
                  {executingPlanId === plan.id ? (
                    <>
                      <RefreshCw size={14} className="animate-spin" />
                      Ausführen...
                    </>
                  ) : (
                    <>
                      <Play size={14} />
                      Ausführen
                    </>
                  )}
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Summary cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground">Aktive Pläne</div>
          <div className="text-2xl font-bold">{activePlans.length}</div>
        </div>
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground">Monatl. Investition</div>
          <div className="text-2xl font-bold">{formatCurrency(monthlyTotal, 'EUR')}</div>
        </div>
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground">Gesamt Pläne</div>
          <div className="text-2xl font-bold">{plans.length}</div>
        </div>
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground">Fällig heute</div>
          <div className="text-2xl font-bold text-amber-600">{duePlans.length}</div>
        </div>
      </div>

      {/* Plans table */}
      <div className="bg-card rounded-lg border border-border">
        <div className="p-4 border-b border-border">
          <h2 className="font-semibold">Alle Sparpläne ({plans.length})</h2>
        </div>

        {plans.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/50">
                  <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
                  <th className="text-left py-3 px-4 font-medium">Portfolio</th>
                  <th className="text-left py-3 px-4 font-medium">Intervall</th>
                  <th className="text-right py-3 px-4 font-medium">Betrag</th>
                  <th className="text-center py-3 px-4 font-medium">Nächste Ausf.</th>
                  <th className="text-center py-3 px-4 font-medium">Ausführungen</th>
                  <th className="text-center py-3 px-4 font-medium">Status</th>
                  <th className="text-right py-3 px-4 font-medium">Aktionen</th>
                </tr>
              </thead>
              <tbody>
                {plans.map((plan) => (
                  <tr
                    key={plan.id}
                    className="border-b border-border last:border-0 hover:bg-muted/30"
                  >
                    <td className="py-3 px-4">
                      <div className="font-medium">{plan.securityName}</div>
                      <div className="text-xs text-muted-foreground">{plan.name}</div>
                    </td>
                    <td className="py-3 px-4 text-muted-foreground">{plan.portfolioName}</td>
                    <td className="py-3 px-4">
                      <span className="text-muted-foreground">
                        {intervalLabels[plan.interval] || plan.interval}
                      </span>
                      <div className="text-xs text-muted-foreground">Tag {plan.dayOfMonth}</div>
                    </td>
                    <td className="py-3 px-4 text-right font-medium">
                      {formatCurrency(plan.amount, plan.currency)}
                    </td>
                    <td className="py-3 px-4 text-center">{formatDate(plan.nextExecution)}</td>
                    <td className="py-3 px-4 text-center">
                      <div className="flex items-center justify-center gap-1">
                        <CheckCircle2 size={14} className="text-green-500" />
                        <span>{plan.executionCount}</span>
                      </div>
                      {plan.totalInvested > 0 && (
                        <div className="text-xs text-muted-foreground">
                          {formatCurrency(plan.totalInvested, plan.currency)}
                        </div>
                      )}
                    </td>
                    <td className="py-3 px-4 text-center">
                      {plan.isActive ? (
                        <span className="px-2 py-0.5 text-xs bg-green-500/10 text-green-600 rounded-full">
                          Aktiv
                        </span>
                      ) : (
                        <span className="px-2 py-0.5 text-xs bg-muted text-muted-foreground rounded-full">
                          Pausiert
                        </span>
                      )}
                    </td>
                    <td className="py-3 px-4">
                      <div className="flex justify-end gap-1">
                        {plan.isActive && duePlans.some((d) => d.id === plan.id) && (
                          <button
                            onClick={() => handleExecute(plan)}
                            disabled={executingPlanId === plan.id}
                            className="p-1.5 hover:bg-green-500/10 rounded-md transition-colors"
                            title="Jetzt ausführen"
                          >
                            {executingPlanId === plan.id ? (
                              <RefreshCw size={16} className="animate-spin text-green-600" />
                            ) : (
                              <Play size={16} className="text-green-600" />
                            )}
                          </button>
                        )}
                        <button
                          onClick={() => handleEdit(plan)}
                          className="p-1.5 hover:bg-muted rounded-md transition-colors"
                          title="Bearbeiten"
                        >
                          <Edit2 size={16} className="text-muted-foreground" />
                        </button>
                        <button
                          onClick={() => handleToggleActive(plan)}
                          className="p-1.5 hover:bg-muted rounded-md transition-colors"
                          title={plan.isActive ? 'Pausieren' : 'Aktivieren'}
                        >
                          {plan.isActive ? (
                            <Pause size={16} className="text-muted-foreground" />
                          ) : (
                            <Play size={16} className="text-green-600" />
                          )}
                        </button>
                        <button
                          onClick={() => handleDelete(plan.id)}
                          className="p-1.5 hover:bg-destructive/10 rounded-md transition-colors"
                          title="Löschen"
                        >
                          <Trash2 size={16} className="text-destructive" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="p-8 text-center text-muted-foreground">
            <CalendarClock className="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Keine Sparpläne vorhanden.</p>
            <p className="text-sm mt-1">Erstellen Sie einen neuen Sparplan.</p>
          </div>
        )}
      </div>

      {/* Investment Plan Form Modal */}
      <InvestmentPlanFormModal
        isOpen={isModalOpen}
        onClose={handleModalClose}
        onSuccess={handleModalSuccess}
        plan={editingPlan}
      />
    </div>
  );
}
