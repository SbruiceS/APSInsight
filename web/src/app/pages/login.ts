import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { Auth } from '../core/auth';
import { TPipe } from '../core/i18n';

@Component({
  selector: 'apsi-login',
  imports: [FormsModule, TPipe],
  template: `
    <section class="narrow">
      <h1 tabindex="-1">{{ 'login.title' | t }}</h1>
      <form (ngSubmit)="submit()">
        <label for="token">{{ 'login.token' | t }}</label>
        <input id="token" name="token" type="password" autocomplete="off" required [(ngModel)]="token" aria-describedby="token-help" />
        <p id="token-help" class="muted">{{ 'login.help' | t }}</p>
        <button type="submit" [disabled]="busy() || !token">{{ 'login.submit' | t }}</button>
      </form>
      @if (failed()) {
        <p class="state state-error" role="alert">{{ 'login.failed' | t }} <code>{{ failed() }}</code></p>
      }
    </section>
  `,
})
export class LoginPage {
  private readonly api = inject(Api);
  private readonly auth = inject(Auth);
  private readonly router = inject(Router);
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
