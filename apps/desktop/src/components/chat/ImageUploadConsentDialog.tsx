/**
 * Image Upload Consent Dialog - Asks for user consent before uploading images to AI.
 *
 * SECURITY: Ensures user is aware that image data will be sent to external AI provider.
 * This dialog must be shown before the first image upload in a session.
 */

import { AlertTriangle, Shield } from 'lucide-react';

interface ImageUploadConsentDialogProps {
  isOpen: boolean;
  providerName: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ImageUploadConsentDialog({
  isOpen,
  providerName,
  onConfirm,
  onCancel,
}: ImageUploadConsentDialogProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={onCancel}
      />

      {/* Dialog */}
      <div className="relative bg-background rounded-lg shadow-xl border border-border max-w-md w-full mx-4 p-6">
        {/* Header */}
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 rounded-full bg-amber-500/10">
            <AlertTriangle className="h-6 w-6 text-amber-500" />
          </div>
          <h2 className="text-lg font-semibold">Bild an KI senden?</h2>
        </div>

        {/* Content */}
        <div className="space-y-4 text-sm text-muted-foreground">
          <p>
            Das Bild wird zur Analyse an <strong className="text-foreground">{providerName}</strong> gesendet.
          </p>

          <div className="p-3 rounded-lg bg-amber-500/10 border border-amber-500/20">
            <div className="flex items-start gap-2">
              <Shield className="h-4 w-4 text-amber-500 mt-0.5 shrink-0" />
              <div>
                <p className="font-medium text-foreground mb-1">Datenschutz-Hinweis</p>
                <ul className="list-disc list-inside space-y-1 text-xs">
                  <li>Bilder können sensible Daten enthalten (Kontonummern, Namen, etc.)</li>
                  <li>Die Daten werden an externe Server übertragen</li>
                  <li>Je nach Anbieter können Daten für Training verwendet werden</li>
                </ul>
              </div>
            </div>
          </div>

          <p className="text-xs">
            Diese Zustimmung gilt für die aktuelle Sitzung. Du kannst sie jederzeit widerrufen,
            indem du den Chat schließt und neu öffnest.
          </p>
        </div>

        {/* Actions */}
        <div className="flex gap-3 mt-6">
          <button
            onClick={onCancel}
            className="flex-1 px-4 py-2 text-sm font-medium rounded-lg border border-border bg-background hover:bg-muted transition-colors"
          >
            Abbrechen
          </button>
          <button
            onClick={onConfirm}
            className="flex-1 px-4 py-2 text-sm font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
          >
            Zustimmen & Senden
          </button>
        </div>
      </div>
    </div>
  );
}

export default ImageUploadConsentDialog;
