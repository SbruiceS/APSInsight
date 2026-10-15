import { HttpErrorResponse, HttpInterceptorFn } from '@angular/common/http';
import { Injectable, computed, inject, signal } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { catchError, throwError } from 'rxjs';
import { Principal, Role } from './models';

const KEY = 'apsi.token';

@Injectable({ providedIn: 'root' })
export class Auth {
  readonly token = signal<string | null>(this.read());
  readonly principal = signal<Principal | null>(null);
  readonly signedIn = computed(() => this.token() !== null);

  private read(): string | null {
    try {
      return sessionStorage.getItem(KEY);
    } catch {
      return null;
    }
  }

  setToken(token: string | null): void {
    this.token.set(token);
    try {
      if (token) sessionStorage.setItem(KEY, token);
      else sessionStorage.removeItem(KEY);
    } catch {
      // session storage unavailable: token lives in memory only
    }
    if (!token) this.principal.set(null);
  }

  has(role: Role): boolean {
    const roles = this.principal()?.roles ?? [];
    return roles.includes('admin') || roles.includes(role);
  }
}

export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const token = inject(Auth).token();
  if (!token || !req.url.startsWith('/api/')) return next(req);
  return next(req.clone({ setHeaders: { Authorization: `Bearer ${token}` } }));
};

export const signedInGuard: CanActivateFn = () => {
  const auth = inject(Auth);
  return auth.signedIn() ? true : inject(Router).createUrlTree(['/login']);
};
