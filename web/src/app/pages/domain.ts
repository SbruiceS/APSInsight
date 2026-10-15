import { Component, computed, effect, inject, input, untracked } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { Auth } from '../core/auth';
import { I18n, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { FeatureStatus } from '../core/models';
import { groupOf, iconOf } from '../core/taxonomy';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';
import { Meter, Stat } from '../shared/widgets';

const STATUS_ORDER: FeatureStatus[] = ['implemented-and-tested', 'implemented-not-fully-tested', 'specified-only', 'blocked-external'];

@Component({
  selector: 'apsi-domain',
  imports: [RouterLink, StateView, TPipe, Icon, Meter, Stat],
  template: `
    <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
    @if (state.data(); as d) {
      <nav aria-label="breadcrumb" class="breadcrumb">
        <a routerLink="/domains">{{ 'nav.domains' | t }}</a>
        <span aria-hidden="true">›</span>
        <a [routerLink]="['/domains']" [queryParams]="{ group: group().id }">{{ group().label | t }}</a>
      </nav>
      <header class="page-head" [attr.data-tone]="group().tone">
        <span class="page-icon"><apsi-icon [name]="icon()" [live]="true" size="xl" /></span>
        <div>
          <h1 tabindex="-1">{{ d.name }}</h1>
          <p class="muted">{{ 'domain.lead' | t: { n: d.entities.length } }}</p>
        </div>
        <div class="page-actions">
          <a class="button-link ghost" [routerLink]="['/features']" [queryParams]="{ domain: domain() }"><apsi-icon name="book" /> {{ 'domain.features' | t }}</a>
          <a class="button-link ghost" routerLink="/lab"><apsi-icon name="flask" /> {{ 'nav.lab' | t }}</a>
        </div>
      </header>

      <div class="stats-grid">
        <apsi-stat icon="layers" [tone]="group().tone" [label]="'domain.total' | t" [value]="i18n.number(d.total)" />
        <apsi-stat icon="sprout" tone="leaf" [label]="'domain.active' | t" [value]="i18n.number(sum('active'))" />
        <apsi-stat icon="seed" tone="wheat" [label]="'domain.draft' | t" [value]="i18n.number(sum('draft'))" />
        <apsi-stat icon="scroll" tone="slate" [label]="'domain.archived' | t" [value]="i18n.number(sum('archived'))" />
        <apsi-stat icon="bell" tone="clay" [label]="'nav.notifications' | t" [value]="i18n.number(d.notifications)" link="/notifications" />
      </div>

      <div class="two-col wide-left">
        <section aria-labelledby="ent-h">
          <h2 id="ent-h">{{ 'domain.entities' | t }}</h2>
          <ul class="entity-grid" role="list">
            @for (e of d.entities; track e.entity) {
              <li class="entity-card">
                <div class="entity-head">
                  <apsi-icon [name]="icon()" />
                  <a [routerLink]="['/domains', d.domain, e.entity]">{{ e.name }}</a>
                  <span class="badge">v{{ e.schema_version }}</span>
                </div>
                <p class="entity-total"><strong>{{ i18n.number(e.total) }}</strong> {{ 'overview.records' | t }}</p>
                <div class="lifecycle-bar" role="img" [attr.aria-label]="'domain.lifecycleLabel' | t: { active: e.active, draft: e.draft, archived: e.archived }">
                  <span class="lc-active" [style.flex-grow]="e.active"></span>
                  <span class="lc-draft" [style.flex-grow]="e.draft"></span>
                  <span class="lc-archived" [style.flex-grow]="e.archived"></span>
                </div>
                <apsi-meter [label]="'domain.validity' | t" [value]="e.validity" />
                <apsi-meter [label]="'domain.completeness' | t" [value]="e.completeness" />
                <div class="entity-actions">
                  <a [routerLink]="['/domains', d.domain, e.entity]">{{ 'domain.open' | t }} <apsi-icon name="arrowRight" /></a>
                  @if (auth.has('editor')) {
                    <a [routerLink]="['/domains', d.domain, e.entity]" fragment="new-h">{{ 'records.new' | t }} <apsi-icon name="plus" /></a>
                  }
                </div>
              </li>
            }
          </ul>
        </section>

        <aside class="side-stack">
          <section class="panel" aria-labelledby="st-h">
            <h2 id="st-h">{{ 'domain.status' | t }}</h2>
            @if (statusCounts(); as sc) {
              <ul class="status-list" role="list">
                @for (s of sc; track s.status) {
                  <li [attr.data-status]="s.status">
                    <span class="status-dot" aria-hidden="true"></span>
                    <span>{{ s.label | t }}</span>
                    <strong>{{ s.n }}</strong>
                  </li>
                }
              </ul>
              <a [routerLink]="['/features']" [queryParams]="{ domain: domain() }" class="small">{{ 'domain.featuresLink' | t }} <apsi-icon name="arrowRight" /></a>
            } @else {
              <apsi-state [status]="status.status()" [error]="status.error()" (retry)="status.reload()" />
            }
          </section>

          <section class="panel" aria-labelledby="act-h">
            <h2 id="act-h">{{ 'domain.activity' | t }}</h2>
            @if (d.recent_activity.length === 0) {
              <p class="muted">{{ 'state.empty' | t }}</p>
            } @else {
              <ol class="timeline" role="list">
                @for (a of d.recent_activity; track a.seq) {
                  <li [attr.data-outcome]="a.outcome">
                    <apsi-icon [name]="a.outcome === 'allowed' ? 'check' : 'alert'" />
                    <div>
                      <code>{{ a.action }}</code> · {{ a.actor }}
                      <p class="muted small"><time [attr.datetime]="a.at">{{ i18n.date(a.at) }}</time> · {{ a.outcome }}</p>
                    </div>
                  </li>
                }
              </ol>
            }
          </section>
        </aside>
      </div>
    }
  `,
})
export class DomainPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  protected readonly auth = inject(Auth);
  readonly domain = input.required<string>();
  readonly state = loadable(() => this.api.dashboard(this.domain()), undefined, false);
  readonly status = loadable(() => this.api.status());
  readonly group = computed(() => groupOf(this.domain()));
  readonly icon = computed(() => iconOf(this.domain()));
  readonly statusCounts = computed(() => {
    const row = this.status.data()?.domains.find((x) => x.domain === this.domain());
    if (!row) return null;
    const labels = { 'implemented-and-tested': 'status.tested', 'implemented-not-fully-tested': 'status.partial', 'specified-only': 'status.specified', 'blocked-external': 'status.blocked' } as const;
    return STATUS_ORDER.map((s) => ({ status: s, label: labels[s], n: row.counts[s] ?? 0 }));
  });

  constructor() {
    effect(() => {
      this.domain();
      untracked(() => this.state.reload());
    });
  }

  sum(key: 'active' | 'draft' | 'archived'): number {
    return (this.state.data()?.entities ?? []).reduce((a, e) => a + e[key], 0);
  }
}
