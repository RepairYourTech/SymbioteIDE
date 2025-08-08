// Accessibility Enhancements for SymbioteIDE
// Comprehensive accessibility features for production readiness

import React, { useState, useEffect, useRef } from 'react';
import './AccessibilityEnhancements.css';

interface AccessibilityContextType {
  highContrast: boolean;
  reducedMotion: boolean;
  fontSize: 'small' | 'medium' | 'large' | 'xl';
  screenReaderMode: boolean;
  keyboardNavigation: boolean;
  focusVisible: boolean;
}

const AccessibilityContext = React.createContext<AccessibilityContextType>({
  highContrast: false,
  reducedMotion: false,
  fontSize: 'medium',
  screenReaderMode: false,
  keyboardNavigation: false,
  focusVisible: true,
});

export const useAccessibility = () => React.useContext(AccessibilityContext);

interface AccessibilityProviderProps {
  children: React.ReactNode;
}

export const AccessibilityProvider: React.FC<AccessibilityProviderProps> = ({ children }) => {
  const [accessibilityState, setAccessibilityState] = useState<AccessibilityContextType>({
    highContrast: false,
    reducedMotion: false,
    fontSize: 'medium',
    screenReaderMode: false,
    keyboardNavigation: false,
    focusVisible: true,
  });

  useEffect(() => {
    // Detect system preferences
    const detectSystemPreferences = () => {
      const highContrast = window.matchMedia('(prefers-contrast: high)').matches;
      const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      
      setAccessibilityState(prev => ({
        ...prev,
        highContrast,
        reducedMotion,
      }));
    };

    // Detect keyboard navigation
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Tab') {
        setAccessibilityState(prev => ({
          ...prev,
          keyboardNavigation: true,
          focusVisible: true,
        }));
      }
    };

    const handleMouseDown = () => {
      setAccessibilityState(prev => ({
        ...prev,
        keyboardNavigation: false,
        focusVisible: false,
      }));
    };

    // Detect screen reader
    const detectScreenReader = () => {
      const screenReader = navigator.userAgent.includes('NVDA') || 
                          navigator.userAgent.includes('JAWS') || 
                          navigator.userAgent.includes('VoiceOver') ||
                          window.speechSynthesis !== undefined;
      
      setAccessibilityState(prev => ({
        ...prev,
        screenReaderMode: screenReader,
      }));
    };

    detectSystemPreferences();
    detectScreenReader();

    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleMouseDown);

    // Listen for system preference changes
    const contrastQuery = window.matchMedia('(prefers-contrast: high)');
    const motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    
    contrastQuery.addEventListener('change', detectSystemPreferences);
    motionQuery.addEventListener('change', detectSystemPreferences);

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('mousedown', handleMouseDown);
      contrastQuery.removeEventListener('change', detectSystemPreferences);
      motionQuery.removeEventListener('change', detectSystemPreferences);
    };
  }, []);

  // Apply accessibility classes to document
  useEffect(() => {
    const classes = [];
    
    if (accessibilityState.highContrast) classes.push('high-contrast');
    if (accessibilityState.reducedMotion) classes.push('reduced-motion');
    if (accessibilityState.screenReaderMode) classes.push('screen-reader');
    if (accessibilityState.keyboardNavigation) classes.push('keyboard-nav');
    if (accessibilityState.focusVisible) classes.push('focus-visible');
    
    classes.push(`font-size-${accessibilityState.fontSize}`);

    document.documentElement.className = classes.join(' ');
  }, [accessibilityState]);

  return (
    <AccessibilityContext.Provider value={accessibilityState}>
      {children}
    </AccessibilityContext.Provider>
  );
};

// Accessibility Settings Panel
interface AccessibilitySettingsProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AccessibilitySettings: React.FC<AccessibilitySettingsProps> = ({ isOpen, onClose }) => {
  const accessibility = useAccessibility();
  const [localSettings, setLocalSettings] = useState(accessibility);

  const updateSetting = <K extends keyof AccessibilityContextType>(
    key: K,
    value: AccessibilityContextType[K]
  ) => {
    setLocalSettings(prev => ({ ...prev, [key]: value }));
    
    // Save to localStorage
    localStorage.setItem('symbiote-accessibility', JSON.stringify({
      ...localSettings,
      [key]: value,
    }));
  };

  if (!isOpen) return null;

  return (
    <div className="accessibility-settings-overlay" role="dialog" aria-labelledby="accessibility-title">
      <div className="accessibility-settings-panel">
        <div className="accessibility-header">
          <h2 id="accessibility-title">♿ Accessibility Settings</h2>
          <button 
            className="close-button"
            onClick={onClose}
            aria-label="Close accessibility settings"
          >
            ✕
          </button>
        </div>

        <div className="accessibility-content">
          <div className="setting-group">
            <h3>Visual Preferences</h3>
            
            <div className="setting-item">
              <label htmlFor="high-contrast">
                <input
                  id="high-contrast"
                  type="checkbox"
                  checked={localSettings.highContrast}
                  onChange={(e) => updateSetting('highContrast', e.target.checked)}
                />
                High Contrast Mode
              </label>
              <p className="setting-description">
                Increases contrast for better visibility
              </p>
            </div>

            <div className="setting-item">
              <label htmlFor="font-size">Font Size</label>
              <select
                id="font-size"
                value={localSettings.fontSize}
                onChange={(e) => updateSetting('fontSize', e.target.value as any)}
              >
                <option value="small">Small</option>
                <option value="medium">Medium</option>
                <option value="large">Large</option>
                <option value="xl">Extra Large</option>
              </select>
              <p className="setting-description">
                Adjust text size for better readability
              </p>
            </div>
          </div>

          <div className="setting-group">
            <h3>Motion Preferences</h3>
            
            <div className="setting-item">
              <label htmlFor="reduced-motion">
                <input
                  id="reduced-motion"
                  type="checkbox"
                  checked={localSettings.reducedMotion}
                  onChange={(e) => updateSetting('reducedMotion', e.target.checked)}
                />
                Reduce Motion
              </label>
              <p className="setting-description">
                Minimizes animations and transitions
              </p>
            </div>
          </div>

          <div className="setting-group">
            <h3>Navigation Preferences</h3>
            
            <div className="setting-item">
              <label htmlFor="keyboard-nav">
                <input
                  id="keyboard-nav"
                  type="checkbox"
                  checked={localSettings.keyboardNavigation}
                  onChange={(e) => updateSetting('keyboardNavigation', e.target.checked)}
                />
                Enhanced Keyboard Navigation
              </label>
              <p className="setting-description">
                Improved focus indicators and keyboard shortcuts
              </p>
            </div>

            <div className="setting-item">
              <label htmlFor="screen-reader">
                <input
                  id="screen-reader"
                  type="checkbox"
                  checked={localSettings.screenReaderMode}
                  onChange={(e) => updateSetting('screenReaderMode', e.target.checked)}
                />
                Screen Reader Optimizations
              </label>
              <p className="setting-description">
                Enhanced compatibility with screen readers
              </p>
            </div>
          </div>

          <div className="accessibility-shortcuts">
            <h3>Keyboard Shortcuts</h3>
            <div className="shortcut-list">
              <div className="shortcut-item">
                <kbd>Alt + A</kbd>
                <span>Open Accessibility Settings</span>
              </div>
              <div className="shortcut-item">
                <kbd>Alt + 1</kbd>
                <span>Focus Main Navigation</span>
              </div>
              <div className="shortcut-item">
                <kbd>Alt + 2</kbd>
                <span>Focus Editor Area</span>
              </div>
              <div className="shortcut-item">
                <kbd>Alt + 3</kbd>
                <span>Focus Side Panel</span>
              </div>
              <div className="shortcut-item">
                <kbd>Esc</kbd>
                <span>Close Current Modal</span>
              </div>
            </div>
          </div>
        </div>

        <div className="accessibility-footer">
          <button onClick={onClose} className="primary-button">
            Apply Settings
          </button>
          <button 
            onClick={() => {
              // Reset to defaults
              const defaults: AccessibilityContextType = {
                highContrast: false,
                reducedMotion: false,
                fontSize: 'medium',
                screenReaderMode: false,
                keyboardNavigation: false,
                focusVisible: true,
              };
              setLocalSettings(defaults);
              localStorage.removeItem('symbiote-accessibility');
            }}
            className="secondary-button"
          >
            Reset to Defaults
          </button>
        </div>
      </div>
    </div>
  );
};

// Skip Navigation Component
export const SkipNavigation: React.FC = () => {
  return (
    <div className="skip-navigation">
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>
      <a href="#navigation" className="skip-link">
        Skip to navigation
      </a>
      <a href="#sidebar" className="skip-link">
        Skip to sidebar
      </a>
    </div>
  );
};

// Focus Trap Component for Modals
interface FocusTrapProps {
  children: React.ReactNode;
  isActive: boolean;
}

export const FocusTrap: React.FC<FocusTrapProps> = ({ children, isActive }) => {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isActive || !containerRef.current) return;

    const container = containerRef.current;
    const focusableElements = container.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );

    const firstElement = focusableElements[0] as HTMLElement;
    const lastElement = focusableElements[focusableElements.length - 1] as HTMLElement;

    const handleTabKey = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;

      if (e.shiftKey) {
        if (document.activeElement === firstElement) {
          lastElement?.focus();
          e.preventDefault();
        }
      } else {
        if (document.activeElement === lastElement) {
          firstElement?.focus();
          e.preventDefault();
        }
      }
    };

    const handleEscapeKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        // Find and trigger close button
        const closeButton = container.querySelector('[aria-label*="close"], .close-button') as HTMLElement;
        closeButton?.click();
      }
    };

    document.addEventListener('keydown', handleTabKey);
    document.addEventListener('keydown', handleEscapeKey);

    // Focus first element when trap becomes active
    firstElement?.focus();

    return () => {
      document.removeEventListener('keydown', handleTabKey);
      document.removeEventListener('keydown', handleEscapeKey);
    };
  }, [isActive]);

  return (
    <div ref={containerRef} className="focus-trap">
      {children}
    </div>
  );
};

// Announcement Component for Screen Readers
interface AnnouncementProps {
  message: string;
  priority: 'polite' | 'assertive';
}

export const Announcement: React.FC<AnnouncementProps> = ({ message, priority }) => {
  return (
    <div
      className="sr-only"
      aria-live={priority}
      aria-atomic="true"
      role="status"
    >
      {message}
    </div>
  );
};

// Loading Indicator with Accessibility
interface AccessibleLoadingProps {
  isLoading: boolean;
  message?: string;
}

export const AccessibleLoading: React.FC<AccessibleLoadingProps> = ({ 
  isLoading, 
  message = 'Loading...' 
}) => {
  if (!isLoading) return null;

  return (
    <div className="accessible-loading" role="status" aria-live="polite">
      <div className="loading-spinner" aria-hidden="true"></div>
      <span className="loading-text">{message}</span>
    </div>
  );
};

export default AccessibilityProvider;
