import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

export type ThemePreference = 'light' | 'dark' | 'system';
type EffectiveTheme = 'light' | 'dark';

type ThemeContextValue = {
  preference: ThemePreference;
  effectiveTheme: EffectiveTheme;
  setPreference: (theme: ThemePreference) => void;
  cycleTheme: () => void;
};

const STORAGE_KEY = 'boldtrace.theme';
const ThemeContext = createContext<ThemeContextValue | null>(null);

function readPreference(): ThemePreference {
  if (typeof window === 'undefined') return 'system';
  const saved = window.localStorage.getItem(STORAGE_KEY);
  return saved === 'light' || saved === 'dark' || saved === 'system' ? saved : 'system';
}

function systemTheme(): EffectiveTheme {
  if (typeof window === 'undefined') return 'dark';
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<ThemePreference>(readPreference);
  const [system, setSystem] = useState<EffectiveTheme>(systemTheme);
  const effectiveTheme: EffectiveTheme = preference === 'system' ? system : preference;

  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: light)');
    const update = () => setSystem(query.matches ? 'light' : 'dark');
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = effectiveTheme;
    document.documentElement.dataset.themePreference = preference;
    document.documentElement.style.colorScheme = effectiveTheme;
  }, [effectiveTheme, preference]);

  const setPreference = (theme: ThemePreference) => {
    window.localStorage.setItem(STORAGE_KEY, theme);
    setPreferenceState(theme);
  };

  const value = useMemo<ThemeContextValue>(() => ({
    preference,
    effectiveTheme,
    setPreference,
    cycleTheme: () => setPreference(preference === 'system' ? 'light' : preference === 'light' ? 'dark' : 'system'),
  }), [preference, effectiveTheme]);

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const value = useContext(ThemeContext);
  if (!value) throw new Error('useTheme must be used inside ThemeProvider');
  return value;
}
