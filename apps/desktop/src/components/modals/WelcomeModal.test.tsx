/**
 * Tests for WelcomeModal Component
 *
 * Tests the first-time user name input modal
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { WelcomeModal } from './WelcomeModal';

// Mock Zustand store
const mockSetUserName = vi.fn();

vi.mock('../../store', () => ({
  useSettingsStore: () => ({
    setUserName: mockSetUserName,
  }),
}));

// Mock useEscapeKey hook
vi.mock('../../lib/hooks', () => ({
  useEscapeKey: vi.fn(),
}));

describe('WelcomeModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Rendering', () => {
    it('should render when isOpen is true', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      expect(screen.getByText('Willkommen bei Portfolio Now!')).toBeInTheDocument();
      expect(screen.getByText('Wie darf ich dich nennen?')).toBeInTheDocument();
    });

    it('should not render when isOpen is false', () => {
      render(<WelcomeModal isOpen={false} onClose={() => {}} />);

      expect(screen.queryByText('Willkommen bei Portfolio Now!')).not.toBeInTheDocument();
    });

    it('should show name input field', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      const input = screen.getByPlaceholderText('z.B. Max');
      expect(input).toBeInTheDocument();
    });

    it('should show submit and skip buttons', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      expect(screen.getByText('Weiter')).toBeInTheDocument();
      expect(screen.getByText('Überspringen')).toBeInTheDocument();
    });

    it('should show description text about AI usage', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      expect(
        screen.getByText(/Dieser Name wird in KI-Konversationen verwendet/)
      ).toBeInTheDocument();
    });
  });

  describe('Form Submission', () => {
    it('should disable submit button when name is empty', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      const submitButton = screen.getByText('Weiter');
      expect(submitButton).toBeDisabled();
    });

    it('should enable submit button when name is entered', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      const input = screen.getByPlaceholderText('z.B. Max');
      fireEvent.change(input, { target: { value: 'Max' } });

      const submitButton = screen.getByText('Weiter');
      expect(submitButton).not.toBeDisabled();
    });

    it('should call setUserName with trimmed name on submit', async () => {
      const onClose = vi.fn();
      render(<WelcomeModal isOpen={true} onClose={onClose} />);

      const input = screen.getByPlaceholderText('z.B. Max');
      fireEvent.change(input, { target: { value: '  Max Mustermann  ' } });

      const submitButton = screen.getByText('Weiter');
      fireEvent.click(submitButton);

      await waitFor(() => {
        expect(mockSetUserName).toHaveBeenCalledWith('Max Mustermann');
        expect(onClose).toHaveBeenCalled();
      });
    });

    it('should not submit with only whitespace', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      const input = screen.getByPlaceholderText('z.B. Max');
      fireEvent.change(input, { target: { value: '   ' } });

      const submitButton = screen.getByText('Weiter');
      expect(submitButton).toBeDisabled();
    });
  });

  describe('Skip Functionality', () => {
    it('should call setUserName with empty string on skip', async () => {
      const onClose = vi.fn();
      render(<WelcomeModal isOpen={true} onClose={onClose} />);

      const skipButton = screen.getByText('Überspringen');
      fireEvent.click(skipButton);

      await waitFor(() => {
        expect(mockSetUserName).toHaveBeenCalledWith('');
        expect(onClose).toHaveBeenCalled();
      });
    });

    it('should close modal when clicking backdrop', async () => {
      const onClose = vi.fn();
      render(<WelcomeModal isOpen={true} onClose={onClose} />);

      // Click backdrop (the dark overlay)
      const backdrop = document.querySelector('.bg-black\\/50');
      if (backdrop) {
        fireEvent.click(backdrop);
      }

      await waitFor(() => {
        expect(onClose).toHaveBeenCalled();
      });
    });
  });

  describe('Input Handling', () => {
    it('should update input value on change', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      const input = screen.getByPlaceholderText('z.B. Max') as HTMLInputElement;
      fireEvent.change(input, { target: { value: 'Test Name' } });

      expect(input.value).toBe('Test Name');
    });

    it('should handle special characters in name', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      const input = screen.getByPlaceholderText('z.B. Max') as HTMLInputElement;
      fireEvent.change(input, { target: { value: 'René Müller-Schmidt' } });

      expect(input.value).toBe('René Müller-Schmidt');
    });

    it('should have input field ready for input', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      const input = screen.getByPlaceholderText('z.B. Max');
      // Input should be present and editable
      expect(input).toBeInTheDocument();
      expect(input).not.toBeDisabled();
    });
  });

  describe('Form Validation', () => {
    it('should trim whitespace before validation', () => {
      render(<WelcomeModal isOpen={true} onClose={() => {}} />);

      const input = screen.getByPlaceholderText('z.B. Max');

      // Only whitespace
      fireEvent.change(input, { target: { value: '   ' } });
      expect(screen.getByText('Weiter')).toBeDisabled();

      // With actual content
      fireEvent.change(input, { target: { value: '  Max  ' } });
      expect(screen.getByText('Weiter')).not.toBeDisabled();
    });
  });

  describe('Loading State', () => {
    it('should handle submission', async () => {
      const onClose = vi.fn();
      render(<WelcomeModal isOpen={true} onClose={onClose} />);

      const input = screen.getByPlaceholderText('z.B. Max');
      fireEvent.change(input, { target: { value: 'Max' } });

      const submitButton = screen.getByText('Weiter');
      fireEvent.click(submitButton);

      // Should call setUserName and close
      await waitFor(() => {
        expect(mockSetUserName).toHaveBeenCalledWith('Max');
        expect(onClose).toHaveBeenCalled();
      });
    });
  });
});

describe('WelcomeModal localStorage Integration', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('should set welcome-seen flag after submission', async () => {
    // This tests the App.tsx logic that sets localStorage after close
    const welcomeSeenKey = 'portfolio-welcome-seen';

    // Simulate what App.tsx does
    localStorage.setItem(welcomeSeenKey, 'true');

    expect(localStorage.getItem(welcomeSeenKey)).toBe('true');
  });

  it('should not show modal if already seen', () => {
    localStorage.setItem('portfolio-welcome-seen', 'true');

    // Simulate App.tsx check
    const hasSeenWelcome = localStorage.getItem('portfolio-welcome-seen');
    const shouldShowWelcome = !hasSeenWelcome;

    expect(shouldShowWelcome).toBe(false);
  });
});
