/**
 * Welcome modal for first-time users to enter their name.
 * The name is used in AI conversations for a personalized experience.
 */

import { useState } from 'react';
import { User } from 'lucide-react';
import { useSettingsStore } from '../../store';
import { useEscapeKey } from '../../lib/hooks';

interface WelcomeModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function WelcomeModal({ isOpen, onClose }: WelcomeModalProps) {
  const { setUserName } = useSettingsStore();
  const [name, setName] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEscapeKey(isOpen, onClose);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setIsSubmitting(true);
    try {
      setUserName(name.trim());
      onClose();
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleSkip = () => {
    setUserName(''); // Explicitly set empty to mark as "seen"
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={handleSkip} />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-lg w-full max-w-md mx-4 overflow-hidden">
        {/* Header */}
        <div className="bg-gradient-to-r from-primary/10 to-primary/5 p-6 text-center">
          <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <User size={32} className="text-primary" />
          </div>
          <h2 className="text-xl font-semibold">Willkommen bei Portfolio Now!</h2>
          <p className="text-muted-foreground mt-2">
            Wie darf ich dich nennen?
          </p>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          <div>
            <label htmlFor="userName" className="block text-sm font-medium mb-2">
              Dein Name
            </label>
            <input
              type="text"
              id="userName"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="z.B. Max"
              autoFocus
              className="w-full px-4 py-3 border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary text-lg"
            />
            <p className="text-xs text-muted-foreground mt-2">
              Dieser Name wird in KI-Konversationen verwendet, um dich persönlich anzusprechen.
            </p>
          </div>

          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={handleSkip}
              className="flex-1 px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors"
            >
              Überspringen
            </button>
            <button
              type="submit"
              disabled={!name.trim() || isSubmitting}
              className="flex-1 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Wird gespeichert...' : 'Weiter'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
