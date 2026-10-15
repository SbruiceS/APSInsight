import { Injectable, signal } from '@angular/core';

export type ThemeChoice = 'system' | 'light' | 'dark';
const KEY = 'apsi.theme';
const ORDER: ThemeChoice[] = ['system', 'light', 'dark'];

/** Light / dark / follow-the-system theme, applied as `data-theme` on <html>. */
@Injectable({ providedIn: 'root' })
export class Theme {
  readonly choice = signal<ThemeChoice>(this.initial());

  constructor() {
    this.apply(this.choice());
  }

  private initial(): ThemeChoice {
    try {
      const saved = localStorage.getItem(KEY) as ThemeChoice | null;
      if (saved && ORDER.includes(saved)) return saved;
    } catch {
      // storage unavailable: follow the system theme
    }
    return 'system';
  }

  cycle(): void {
    const next = ORDER[(ORDER.indexOf(this.choice()) + 1) % ORDER.length];
    this.choice.set(next);
    this.apply(next);
    try {
      localStorage.setItem(KEY, next);
    } catch {
      // preference simply won't persist
    }
  }

  private apply(choice: ThemeChoice): void {
    if (choice === 'system') document.documentElement.removeAttribute('data-theme');
    else document.documentElement.setAttribute('data-theme', choice);
  }
}
