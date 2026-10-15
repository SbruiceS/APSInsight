import { Component, computed, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { Auth } from '../core/auth';
import { I18n, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { GROUPS, groupOf, iconOf } from '../core/taxonomy';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';
import { Landscape, Stat } from '../shared/widgets';

@Component({
  selector: 'apsi-overview',
  imports: [RouterLink, StateView, TPipe, Icon, Landscape, Stat],
  template: `
    <section class="hero">
      <apsi-landscape />
      <div class="hero-text">
        <p class="eyebrow">{{ 'overview.eyebrow' | t }}</p>
        <h1 tabindex="-1">{{ 'overview.title' | t }}</h1>
        <p>{{ 'overview.lead' | t: { tenant: auth.principal()?.tenant ?? '…' } }}</p>
      </div>
    </section>

    <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
    @if (state.data(); as o) {
      <div class="stats-grid">
        <apsi-stat icon="wheat" tone="leaf" [label]="'overview.records' | t" [value]="i18n.number(o.records)" link="/domains" />
        <apsi-stat icon="field" tone="soil" [label]="'overview.domains' | t" [value]="i18n.number(o.domains.length)" link="/domains" />
        <apsi-stat icon="book" tone="sky" [label]="'overview.entityKinds' | t" [value]="i18n.number(entityKinds())" link="/features" />
        <apsi-stat
          icon="shield"
          [tone]="o.audit_chain_valid ? 'leaf' : 'clay'"
          [label]="'overview.audit' | t"
          [value]="(o.audit_chain_valid ? 'overview.auditOk' : 'overview.auditBroken') | t"
          [hint]="('overview.auditEvents' | t: { n: i18n.number(o.audit_events) })"
          [link]="auth.has('auditor') ? '/audit' : null"
        />
        @if (status.data(); as s) {
          <apsi-stat icon="gauge" tone="wheat" [label]="'overview.tested' | t" [value]="testedPct()" [hint]="'overview.testedHint' | t: { n: s.total }" link="/status" />
        }
      </div>

      <section class="panel" aria-labelledby="qa-h">
        <h2 id="qa-h">{{ 'overview.quick' | t }}</h2>
        <div class="quick-actions">
          <a class="action-card" routerLink="/domains"><apsi-icon name="field" size="lg" /><strong>{{ 'overview.qaDomains' | t }}</strong><span>{{ 'overview.qaDomainsHint' | t }}</span></a>
          <a class="action-card" routerLink="/lab"><apsi-icon name="flask" size="lg" /><strong>{{ 'overview.qaLab' | t }}</strong><span>{{ 'overview.qaLabHint' | t }}</span></a>
          <a class="action-card" routerLink="/status"><apsi-icon name="gauge" size="lg" /><strong>{{ 'overview.qaStatus' | t }}</strong><span>{{ 'overview.qaStatusHint' | t }}</span></a>
          <a class="action-card" routerLink="/integrations"><apsi-icon name="network" size="lg" /><strong>{{ 'overview.qaAspf' | t }}</strong><span>{{ 'overview.qaAspfHint' | t }}</span></a>
        </div>
      </section>

      <div class="two-col">
        <section class="panel" aria-labelledby="groups-h">
          <h2 id="groups-h">{{ 'overview.groups' | t }}</h2>
          <ul class="group-list" role="list">
            @for (g of groups(); track g.id) {
              <li>
                <a class="group-row" [routerLink]="['/domains']" [queryParams]="{ group: g.id }" [attr.data-tone]="g.tone">
                  <apsi-icon [name]="g.icon" />
                  <span class="group-name">{{ g.label | t }}</span>
                  <span class="muted">{{ 'overview.groupDomains' | t: { n: g.domains.length } }}</span>
                  <strong>{{ i18n.number(g.records) }}</strong>
                </a>
              </li>
            }
          </ul>
        </section>

        <section class="panel" aria-labelledby="top-h">
          <h2 id="top-h">{{ 'overview.top' | t }}</h2>
          @if (top().length === 0) {
            <div class="empty-hint">
              <apsi-icon name="seed" [live]="true" size="xl" />
              <p>{{ 'overview.noRecords' | t }}</p>
              <a routerLink="/domains" class="button-link">{{ 'overview.qaDomains' | t }}</a>
            </div>
          } @else {
            <ol class="bar-list" role="list">
              @for (d of top(); track d.domain) {
                <li>
                  <a [routerLink]="['/domains', d.domain]" [attr.data-tone]="tone(d.domain)">
                    <apsi-icon [name]="icon(d.domain)" />
                    <span class="bar-name">{{ d.name }}</span>
                    <span class="bar" aria-hidden="true"><span [style.width.%]="(d.records / topMax()) * 100"></span></span>
                    <strong>{{ i18n.number(d.records) }}</strong>
                  </a>
                </li>
              }
            </ol>
          }
        </section>
      </div>

      <section class="panel" aria-labelledby="notif-h">
        <div class="panel-head">
          <h2 id="notif-h">{{ 'overview.latest' | t }}</h2>
          <a routerLink="/notifications">{{ 'overview.all' | t }} <apsi-icon name="arrowRight" /></a>
        </div>
        @if ((notes.data() ?? []).length === 0) {
          <p class="muted">{{ 'overview.noNotifications' | t }}</p>
        } @else {
          <ol class="timeline" role="list">
            @for (n of (notes.data() ?? []).slice(0, 5); track n.id) {
              <li>
                <apsi-icon name="bell" />
                <div>
                  <a [routerLink]="['/records', n.record_id]">{{ n.message }}</a>
                  <p class="muted small"><time [attr.datetime]="n.at">{{ i18n.date(n.at) }}</time> · <code>{{ n.event }}</code></p>
                </div>
              </li>
            }
          </ol>
        }
      </section>
    }
  `,
})
export class OverviewPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  protected readonly auth = inject(Auth);
  readonly state = loadable(() => this.api.overview());
  readonly status = loadable(() => this.api.status());
  readonly notes = loadable(() => this.api.notifications(), (v) => v.length === 0);

  readonly entityKinds = computed(() => (this.state.data()?.domains ?? []).reduce((a, d) => a + d.entity_kinds, 0));
  readonly groups = computed(() => {
    const byDomain = new Map((this.state.data()?.domains ?? []).map((d) => [d.domain, d.records]));
    return GROUPS.map((g) => ({ ...g, records: g.domains.reduce((a, d) => a + (byDomain.get(d) ?? 0), 0) }));
  });
  readonly top = computed(() =>
    [...(this.state.data()?.domains ?? [])]
      .filter((d) => d.records > 0)
      .sort((a, b) => b.records - a.records)
      .slice(0, 8),
  );
  readonly topMax = computed(() => Math.max(1, ...this.top().map((d) => d.records)));
  readonly testedPct = computed(() => {
    const s = this.status.data();
    if (!s) return '—';
    const tested = s.by_status.find((x) => x.status === 'implemented-and-tested')?.features ?? 0;
    return `${this.i18n.number((tested / s.total) * 100, 1)}%`;
  });

  icon(d: string) {
    return iconOf(d);
  }

  tone(d: string) {
    return groupOf(d).tone;
  }
}
