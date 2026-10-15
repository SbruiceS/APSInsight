import { Component, inject, signal, viewChild } from '@angular/core';
import { NavigationEnd, Router, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { filter } from 'rxjs';
import { Api } from './core/api.service';
import { Auth } from './core/auth';
import { CatalogStore } from './core/catalog-store';
import { I18n, I18nKey, LOCALES, TPipe } from './core/i18n';
import { Announcer } from './core/load';
import { Theme, ThemeChoice } from './core/theme';
import { Icon } from './shared/icon';
import { NAV_SECTIONS, Palette } from './shared/palette';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, RouterLink, RouterLinkActive, TPipe, Icon, Palette],
  template: `
    <a class="skip" href="#main" (click)="focusMain($event)">{{ 'app.skip' | t }}</a>
    @if (auth.signedIn()) {
      <div class="shell" [class.nav-open]="navOpen()">
        <aside class="sidebar" id="sidebar">
          <a class="brand" routerLink="/" (click)="navOpen.set(false)">
            <span class="brand-mark"><apsi-icon name="sprout" [live]="true" size="lg" /></span>
            <span class="brand-text">
              <strong>{{ 'app.title' | t }}</strong>
              <small>{{ 'app.tagline' | t }}</small>
            </span>
          </a>
          <nav [attr.aria-label]="'nav.label' | t">
            @for (section of nav; track section.label) {
              <p class="nav-heading">{{ section.label | t }}</p>
              <ul role="list">
                @for (p of section.pages; track p.path) {
                  @if (p.path !== '/audit' || auth.has('auditor')) {
                    <li>
                      <a
                        class="nav-link"
                        [routerLink]="p.path"
                        routerLinkActive="active"
                        [routerLinkActiveOptions]="{ exact: p.path === '/' }"
                        ariaCurrentWhenActive="page"
                        (click)="navOpen.set(false)"
                      >
                        <apsi-icon [name]="p.icon" />
                        <span>{{ p.label | t }}</span>
                      </a>
                    </li>
                  }
                }
              </ul>
            }
          </nav>
          @if (auth.principal(); as p) {
            <div class="sidebar-foot">
              <apsi-icon name="users" />
              <span>
                <strong>{{ p.subject }}</strong>
                <small>{{ p.tenant }} · {{ p.roles.join(', ') }}</small>
              </span>
            </div>
          }
        </aside>
        <div class="scrim" (click)="navOpen.set(false)" aria-hidden="true"></div>
        <div class="workspace">
          <header class="topbar">
            <button type="button" class="icon-btn menu-toggle" aria-controls="sidebar" [attr.aria-expanded]="navOpen()" (click)="navOpen.set(!navOpen())">
              <apsi-icon name="menu" />
              <span class="visually-hidden">{{ 'app.menu' | t }}</span>
            </button>
            <button type="button" class="search-trigger" (click)="palette()?.show()">
              <apsi-icon name="search" />
              <span>{{ 'palette.trigger' | t }}</span>
              <kbd>Ctrl K</kbd>
            </button>
            <span class="spacer"></span>
            <button type="button" class="icon-btn" (click)="theme.cycle()" [attr.aria-label]="'theme.toggle' | t" [title]="themeLabel() | t">
              <apsi-icon [name]="theme.choice() === 'dark' ? 'moon' : 'sun'" />
              <span class="theme-name">{{ themeLabel() | t }}</span>
            </button>
            <label for="locale" class="visually-hidden">{{ 'app.language' | t }}</label>
            <span class="locale-wrap">
              <apsi-icon name="globe" />
              <select id="locale" [value]="i18n.locale()" (change)="i18n.setLocale($any($event.target).value)">
                @for (l of locales; track l.code) {
                  <option [value]="l.code">{{ l.label }}</option>
                }
              </select>
            </span>
            <button type="button" class="ghost" (click)="signOut()">
              <apsi-icon name="logout" />
              <span>{{ 'app.signOut' | t }}</span>
            </button>
          </header>
          <main id="main" tabindex="-1">
            <router-outlet />
          </main>
          <footer class="site-foot muted">{{ 'app.footer' | t }}</footer>
        </div>
      </div>
      <apsi-palette />
    } @else {
      <main id="main" tabindex="-1" class="bare">
        <router-outlet />
      </main>
    }
    <div class="toasts" aria-hidden="true">
      @for (t of announcer.toasts(); track t.id) {
        <div class="toast" [attr.data-kind]="t.kind">
          <apsi-icon [name]="t.kind === 'error' ? 'alert' : t.kind === 'success' ? 'check' : 'leaf'" />
          <span>{{ t.text }}</span>
          <button type="button" class="icon-btn" tabindex="-1" (click)="announcer.dismiss(t.id)"><apsi-icon name="close" /></button>
        </div>
      }
    </div>
    <div class="visually-hidden" aria-live="polite" role="status">{{ announcer.message() }}</div>
  `,
})
export class App {
  protected readonly auth = inject(Auth);
  protected readonly i18n = inject(I18n);
  protected readonly theme = inject(Theme);
  protected readonly announcer = inject(Announcer);
  protected readonly locales = LOCALES;
  protected readonly nav = NAV_SECTIONS;
  protected readonly navOpen = signal(false);
  protected readonly palette = viewChild(Palette);
  private readonly router = inject(Router);
  private readonly api = inject(Api);
  private readonly store = inject(CatalogStore);

  constructor() {
    document.documentElement.lang = this.i18n.locale();
    if (this.auth.signedIn()) {
      this.api.me().subscribe({ next: (p) => this.auth.principal.set(p), error: () => undefined });
      this.store.ensure();
    }
    // Move focus to the new page heading after navigation so keyboard and
    // screen-reader users land on the content that changed.
    this.router.events.pipe(filter((e) => e instanceof NavigationEnd)).subscribe(() => {
      setTimeout(() => (document.querySelector('main h1') as HTMLElement | null)?.focus(), 0);
    });
  }

  private readonly themeLabels: Record<ThemeChoice, I18nKey> = { system: 'theme.system', light: 'theme.light', dark: 'theme.dark' };

  protected themeLabel(): I18nKey {
    return this.themeLabels[this.theme.choice()];
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
