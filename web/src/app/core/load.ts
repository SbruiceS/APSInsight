import { Injectable, Signal, signal } from '@angular/core';
import { Observable } from 'rxjs';
import { ApiFailure } from './api.service';

export type LoadStatus = 'loading' | 'ready' | 'empty' | 'error' | 'denied';

export interface Loadable<T> {
  readonly status: Signal<LoadStatus>;
  readonly data: Signal<T | null>;
  readonly error: Signal<ApiFailure | null>;
  reload(): void;
}

/**
 * Wraps a request factory in explicit loading / ready / empty / error /
 * permission-denied states so every view renders all of them. Pass
 * `immediate = false` when the factory depends on component inputs and call
 * `reload()` from an effect instead.
 */
export function loadable<T>(factory: () => Observable<T>, isEmpty: (v: T) => boolean = () => false, immediate = true): Loadable<T> {
  const status = signal<LoadStatus>('loading');
  const data = signal<T | null>(null);
  const error = signal<ApiFailure | null>(null);
  const reload = () => {
    status.set('loading');
    error.set(null);
    factory().subscribe({
      next: (v) => {
        data.set(v);
        status.set(isEmpty(v) ? 'empty' : 'ready');
      },
      error: (e: unknown) => {
        const f = ApiFailure.from(e);
        error.set(f);
        status.set(f.denied ? 'denied' : 'error');
      },
    });
  };
  if (immediate) reload();
  return { status, data, error, reload };
}

/** Polite live region for status announcements (screen readers). */
@Injectable({ providedIn: 'root' })
export class Announcer {
  readonly message = signal('');

  say(text: string): void {
    this.message.set('');
    queueMicrotask(() => this.message.set(text));
  }
}
