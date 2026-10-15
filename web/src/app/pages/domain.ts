import { Component, effect, inject, input, untracked } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { I18n, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { StateView } from '../shared/state-view';

@Component({
  selector: 'apsi-domain',
  imports: [RouterLink, StateView, TPipe],
  template: `
    <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
    @if (state.data(); as d) {
      <h1 tabindex="-1">{{ d.name }}</h1>
      <table>
        <caption>{{ 'domain.entities' | t }} · {{ 'domain.total' | t }} {{ i18n.number(d.total) }}</caption>
        <thead>
          <tr>
            <th scope="col">{{ 'domain.entities' | t }}</th>
            <th scope="col">{{ 'domain.total' | t }}</th>
            <th scope="col">{{ 'domain.active' | t }}</th>
            <th scope="col">{{ 'domain.draft' | t }}</th>
            <th scope="col">{{ 'domain.archived' | t }}</th>
            <th scope="col">{{ 'domain.validity' | t }}</th>
            <th scope="col">{{ 'domain.completeness' | t }}</th>
          </tr>
        </thead>
        <tbody>
          @for (e of d.entities; track e.entity) {
            <tr>
              <th scope="row"><a [routerLink]="['/domains', d.domain, e.entity]">{{ e.name }}</a></th>
              <td>{{ e.total }}</td>
              <td>{{ e.active }}</td>
              <td>{{ e.draft }}</td>
              <td>{{ e.archived }}</td>
              <td>{{ pct(e.validity) }}</td>
              <td>{{ pct(e.completeness) }}</td>
            </tr>
          }
        </tbody>
      </table>
      <h2>{{ 'domain.activity' | t }}</h2>
      @if (d.recent_activity.length === 0) {
        <p class="muted">{{ 'state.empty' | t }}</p>
      } @else {
        <ol class="activity">
          @for (a of d.recent_activity; track a.seq) {
            <li><time [attr.datetime]="a.at">{{ i18n.date(a.at) }}</time> · {{ a.actor }} · <code>{{ a.action }}</code> · {{ a.outcome }}</li>
          }
        </ol>
      }
    }
  `,
})
export class DomainPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  readonly domain = input.required<string>();
  readonly state = loadable(() => this.api.dashboard(this.domain()), undefined, false);

  constructor() {
    effect(() => {
      this.domain();
      untracked(() => this.state.reload());
    });
  }

  pct(v: number | null): string {
    return v === null ? '—' : `${this.i18n.number(v * 100, 1)}%`;
  }
}
