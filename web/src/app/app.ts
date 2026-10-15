import { Component, inject } from '@angular/core';
import { NavigationEnd, Router, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { filter } from 'rxjs';
import { Api } from './core/api.service';
import { Auth } from './core/auth';
import { I18n, LOCALES, TPipe } from './core/i18n';
import { Announcer } from './core/load';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, RouterLink, RouterLinkActive, TPipe],
  template: `
    <a class="skip" href="#main" (click)="focusMain($event)">{{ 'app.skip' | t }}</a>
    <header class="top">
      <a class="brand" routerLink="/">{{ 'app.title' | t }}</a>
      @if (auth.signedIn()) {
        <nav [attr.aria-label]="'nav.label' | t">
          <a routerLink="/" routerLinkActive="active" [routerLinkActiveOptions]="{ exact: true }" ariaCurrentWhenActive="page">{{ 'nav.overview' | t }}</a>
          <a routerLink="/features" routerLinkActive="active" ariaCurrentWhenActive="page">{{ 'nav.features' | t }}</a>
          <a routerLink="/notifications" routerLinkActive="active" ariaCurrentWhenActive="page">{{ 'nav.notifications' | t }}</a>
          @if (auth.has('auditor')) {
            <a routerLink="/audit" routerLinkActive="active" ariaCurrentWhenActive="page">{{ 'nav.audit' | t }}</a>
          }
        </nav>
      }
      <span class="spacer"></span>
      <label for="locale" class="visually-hidden">{{ 'app.language' | t }}</label>
      <select id="locale" [value]="i18n.locale()" (change)="i18n.setLocale($any($event.target).value)">
        @for (l of locales; track l.code) {
          <option [value]="l.code">{{ l.label }}</option>
        }
      </select>
      @if (auth.principal(); as p) {
        <span class="muted">{{ p.subject }} @ {{ p.tenant }} ({{ p.roles.join(', ') }})</span>
      }
      @if (auth.signedIn()) {
        <button type="button" (click)="signOut()">{{ 'app.signOut' | t }}</button>
      }
    </header>
    <main id="main" tabindex="-1">
      <router-outlet />
    </main>
    <div class="visually-hidden" aria-live="polite" role="status">{{ announcer.message() }}</div>
  `,
})
export class App {
  protected readonly auth = inject(Auth);
  protected readonly i18n = inject(I18n);
  protected readonly announcer = inject(Announcer);
  protected readonly locales = LOCALES;
  private readonly router = inject(Router);
  private readonly api = inject(Api);

  constructor() {
    document.documentElement.lang = this.i18n.locale();
    if (this.auth.signedIn()) {
      this.api.me().subscribe({ next: (p) => this.auth.principal.set(p), error: () => undefined });
    }
    // Move focus to the new page heading after navigation so keyboard and
    // screen-reader users land on the content that changed.
    this.router.events.pipe(filter((e) => e instanceof NavigationEnd)).subscribe(() => {
      setTimeout(() => (document.querySelector('main h1') as HTMLElement | null)?.focus(), 0);
    });
  }

  focusMain(e: Event): void {
    e.preventDefault();
    document.getElementById('main')?.focus();
  }

  signOut(): void {
    this.auth.setToken(null);
    void this.router.navigate(['/login']);
  }
}
