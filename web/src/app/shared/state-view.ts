import { Component, input, output } from '@angular/core';
import { ApiFailure } from '../core/api.service';
import { TPipe } from '../core/i18n';
import { LoadStatus } from '../core/load';

/** Renders the non-ready states of a view: loading, empty, error, permission denied. */
@Component({
  selector: 'apsi-state',
  imports: [TPipe],
  template: `
    @switch (status()) {
      @case ('loading') {
        <p class="state" role="status" aria-busy="true">{{ 'state.loading' | t }}</p>
      }
      @case ('empty') {
        <p class="state state-empty" role="status">{{ 'state.empty' | t }}</p>
      }
      @case ('denied') {
        <div class="state state-denied" role="alert">
          <p>{{ 'state.denied' | t }}</p>
          @if (error(); as e) {
            <p class="muted"><code>{{ e.code }}</code></p>
          }
        </div>
      }
      @case ('error') {
        <div class="state state-error" role="alert">
          <p>{{ 'state.error' | t }}</p>
          @if (error(); as e) {
            <p class="muted"><code>{{ e.code }}</code> {{ e.message }}</p>
          }
          <button type="button" (click)="retry.emit()">{{ 'state.retry' | t }}</button>
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
