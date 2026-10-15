import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { Auth } from '../core/auth';
import { CatalogStore } from '../core/catalog-store';
import { I18n, LOCALES, TPipe } from '../core/i18n';
import { Icon } from '../shared/icon';
import { Logo } from '../shared/logo';
import { Landscape } from '../shared/widgets';

@Component({
  selector: 'apsi-login',
  imports: [FormsModule, TPipe, Icon, Landscape, Logo],
  template: `
    <div class="login">
      <section class="login-hero" aria-labelledby="login-tagline">
        <apsi-landscape />
        <div class="login-hero-text">
          <p class="eyebrow"><apsi-icon name="sprout" [live]="true" /> {{ 'app.title' | t }}</p>
          <h2 id="login-tagline">{{ 'login.hero' | t }}</h2>
          <ul class="login-points" role="list">
            <li><apsi-icon name="soil" [live]="true" /> {{ 'login.point1' | t }}</li>
            <li><apsi-icon name="satellite" [live]="true" /> {{ 'login.point2' | t }}</li>
            <li><apsi-icon name="flask" [live]="true" /> {{ 'login.point3' | t }}</li>
          </ul>
        </div>
      </section>
      <section class="login-panel">
        <div class="login-card">
          <apsi-logo class="login-logo" [animated]="true" [label]="'app.title' | t" />
          <h1 tabindex="-1">{{ 'login.title' | t }}</h1>
          <p class="muted">{{ 'login.lead' | t }}</p>
          <form (ngSubmit)="submit()" class="stack">
            <label for="token">{{ 'login.token' | t }}</label>
            <input id="token" name="token" type="password" autocomplete="off" required [(ngModel)]="token" aria-describedby="token-help" />
            <p id="token-help" class="muted small">{{ 'login.help' | t }}</p>
            <button type="submit" class="primary" [disabled]="busy() || !token">
              <apsi-icon name="arrowRight" [live]="busy()" />
              {{ 'login.submit' | t }}
            </button>
          </form>
          @if (failed()) {
            <p class="state state-error" role="alert"><apsi-icon name="alert" /> {{ 'login.failed' | t }} <code>{{ failed() }}</code></p>
          }
          <div class="login-locale">
            <label for="login-locale">{{ 'app.language' | t }}</label>
            <select id="login-locale" [value]="i18n.locale()" (change)="i18n.setLocale($any($event.target).value)">
              @for (l of locales; track l.code) {
                <option [value]="l.code">{{ l.label }}</option>
              }
            </select>
          </div>
        </div>
      </section>
    </div>
  `,
})
export class LoginPage {
  private readonly api = inject(Api);
  private readonly auth = inject(Auth);
  private readonly router = inject(Router);
  private readonly store = inject(CatalogStore);
  protected readonly i18n = inject(I18n);
  protected readonly locales = LOCALES;
  token = '';
  readonly busy = signal(false);
  readonly failed = signal('');

  submit(): void {
    this.busy.set(true);
    this.failed.set('');
    this.auth.setToken(this.token.trim());
    this.api.me().subscribe({
      next: (p) => {
        this.auth.principal.set(p);
        this.busy.set(false);
        this.store.ensure();
        void this.router.navigate(['/']);
      },
      error: (e: unknown) => {
        this.auth.setToken(null);
        this.busy.set(false);
        this.failed.set(ApiFailure.from(e).code);
      },
    });
  }
}
