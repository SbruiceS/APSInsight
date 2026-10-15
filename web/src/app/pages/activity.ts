import { Component, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { I18n, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { StateView } from '../shared/state-view';

@Component({
  selector: 'apsi-notifications',
  imports: [RouterLink, StateView, TPipe],
  template: `
    <h1 tabindex="-1">{{ 'notifications.title' | t }}</h1>
    <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
    @if (state.status() === 'ready') {
      <ol class="activity">
        @for (n of state.data() ?? []; track n.id) {
          <li>
            <time [attr.datetime]="n.at">{{ i18n.date(n.at) }}</time> · <code>{{ n.event }}</code> ·
            <a [routerLink]="['/records', n.record_id]">{{ n.message }}</a>
          </li>
        }
      </ol>
    }
  `,
})
export class NotificationsPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  readonly state = loadable(() => this.api.notifications(), (v) => v.length === 0);
}

@Component({
  selector: 'apsi-audit',
  imports: [StateView, TPipe],
  template: `
    <h1 tabindex="-1">{{ 'audit.title' | t }}</h1>
    <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
    @if (state.status() === 'ready') {
      <table>
        <caption class="visually-hidden">{{ 'audit.title' | t }}</caption>
        <thead>
          <tr>
            <th scope="col">#</th>
            <th scope="col">{{ 'audit.at' | t }}</th>
            <th scope="col">{{ 'audit.actor' | t }}</th>
            <th scope="col">{{ 'audit.action' | t }}</th>
            <th scope="col">{{ 'audit.target' | t }}</th>
            <th scope="col">{{ 'audit.outcome' | t }}</th>
          </tr>
        </thead>
        <tbody>
          @for (e of state.data() ?? []; track e.seq) {
            <tr [class.denied]="e.outcome !== 'allowed'">
              <td>{{ e.seq }}</td>
              <td><time [attr.datetime]="e.at">{{ i18n.date(e.at) }}</time></td>
              <td>{{ e.actor }}</td>
              <td><code>{{ e.action }}</code></td>
              <td><code>{{ e.target }}</code></td>
              <td>{{ e.outcome }}@if (e.detail) { ({{ e.detail }}) }</td>
            </tr>
          }
        </tbody>
      </table>
    }
  `,
})
export class AuditPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  readonly state = loadable(() => this.api.audit(), (v) => v.length === 0);
}
