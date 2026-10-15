import { Component, input, output } from '@angular/core';
import { ApiFailure } from '../core/api.service';
import { TPipe } from '../core/i18n';
import { LoadStatus } from '../core/load';
import { Icon } from './icon';

/** Renders the non-ready states of a view: loading, empty, error, permission denied. */
@Component({
  selector: 'apsi-state',
  imports: [Icon, TPipe],
  template: `
    @switch (status()) {
      @case ('loading') {
        <p class="state state-loading" role="status" aria-busy="true"><apsi-icon name="sprout" [live]="true" size="lg" /> {{ 'state.loading' | t }}</p>
      }
      @case ('empty') {
        <p class="state state-empty" role="status"><apsi-icon name="seed" [live]="true" size="lg" /> {{ 'state.empty' | t }}</p>
      }
      @case ('denied') {
        <div class="state state-denied" role="alert">
          <apsi-icon name="lock" size="lg" />
          <div>
            <p>{{ 'state.denied' | t }}</p>
            @if (error(); as e) {
              <p class="muted"><code>{{ e.code }}</code></p>
            }
          </div>
        </div>
      }
      @case ('error') {
        <div class="state state-error" role="alert">
          <apsi-icon name="alert" size="lg" />
          <div>
            <p>{{ 'state.error' | t }}</p>
            @if (error(); as e) {
              <p class="muted"><code>{{ e.code }}</code> {{ e.message }}</p>
            }
            <button type="button" (click)="retry.emit()">{{ 'state.retry' | t }}</button>
          </div>
        </div>
      }
    }
  `,
})
export class StateView {
  readonly status = input.required<LoadStatus>();
  readonly error = input<ApiFailure | null>(null);
  readonly retry = output<void>();
}
