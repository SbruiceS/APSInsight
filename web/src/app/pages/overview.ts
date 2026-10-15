import { Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { I18n, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { StateView } from '../shared/state-view';

@Component({
  selector: 'apsi-overview',
  imports: [RouterLink, StateView, TPipe],
  template: `
    <h1 tabindex="-1">{{ 'overview.title' | t }}</h1>
    <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
    @if (state.data(); as o) {
      <p>
        {{ 'overview.summary' | t: { records: i18n.number(o.records), domains: o.domains.length } }}
        ·
        <span [class]="o.audit_chain_valid ? 'badge ok' : 'badge bad'">{{ (o.audit_chain_valid ? 'overview.auditOk' : 'overview.auditBroken') | t }}</span>
      </p>
      <label for="domain-filter">{{ 'overview.filter' | t }}</label>
      <input id="domain-filter" type="search" [value]="filter()" (input)="filter.set($any($event.target).value)" />
      <ul class="cards" role="list">
        @for (d of visible(); track d.domain) {
          <li class="card">
            <a [routerLink]="['/domains', d.domain]">{{ d.name }}</a>
            <span class="muted">{{ 'overview.records' | t }}: {{ i18n.number(d.records) }} · {{ 'overview.entityKinds' | t }}: {{ d.entity_kinds }}</span>
          </li>
        }
      </ul>
    }
  `,
})
export class OverviewPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  readonly state = loadable(() => this.api.overview());
  readonly filter = signal('');
  readonly visible = computed(() => {
    const f = this.filter().toLowerCase();
    return (this.state.data()?.domains ?? []).filter((d) => d.name.toLowerCase().includes(f) || d.domain.includes(f));
  });
}
