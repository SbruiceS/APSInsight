import { Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { I18n, I18nKey, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { FeatureStatus } from '../core/models';
import { GROUPS, groupOf, iconOf } from '../core/taxonomy';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';
import { Donut, DonutSlice } from '../shared/widgets';

export const STATUSES: { status: FeatureStatus; label: I18nKey; tone: string }[] = [
  { status: 'implemented-and-tested', label: 'status.tested', tone: 'leaf' },
  { status: 'implemented-not-fully-tested', label: 'status.partial', tone: 'wheat' },
  { status: 'specified-only', label: 'status.specified', tone: 'slate' },
  { status: 'blocked-external', label: 'status.blocked', tone: 'clay' },
];

/** Honest completion: every feature is counted under exactly one of four statuses. */
@Component({
  selector: 'apsi-status',
  imports: [RouterLink, StateView, TPipe, Icon, Donut],
  template: `
    <header class="page-head" data-tone="wheat">
      <span class="page-icon"><apsi-icon name="gauge" [live]="true" size="xl" /></span>
      <div>
        <p class="eyebrow">{{ 'nav.governance' | t }}</p>
        <h1 tabindex="-1">{{ 'status.title' | t }}</h1>
        <p class="muted">{{ 'status.lead' | t }}</p>
      </div>
      <div class="page-actions">
        <a class="button-link ghost" routerLink="/features"><apsi-icon name="book" /> {{ 'nav.features' | t }}</a>
      </div>
    </header>

    <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
    @if (state.data(); as s) {
      <div class="two-col">
        <section class="panel" aria-labelledby="sum-h">
          <h2 id="sum-h">{{ 'status.summary' | t }}</h2>
          <apsi-donut [slices]="slices()" [caption]="'status.chart' | t: { n: s.total }" [centre]="testedPct()" [centreLabel]="'status.testedShort' | t" />
        </section>
        <section class="panel" aria-labelledby="grp-h">
          <h2 id="grp-h">{{ 'status.byGroup' | t }}</h2>
          <ul class="stack-bars" role="list">
            @for (g of groups(); track g.id) {
              <li>
                <a [routerLink]="['/domains']" [queryParams]="{ group: g.id }" class="stack-row" [attr.data-tone]="g.tone">
                  <apsi-icon [name]="g.icon" />
                  <span class="bar-name">{{ g.label | t }}</span>
                  <span class="stacked" role="img" [attr.aria-label]="describe(g.counts)">
                    @for (st of statuses; track st.status) {
                      <span [attr.data-status]="st.status" [style.flex-grow]="g.counts[st.status]"></span>
                    }
                  </span>
                  <strong>{{ pct(g.counts['implemented-and-tested'], g.total) }}</strong>
                </a>
              </li>
            }
          </ul>
        </section>
      </div>

      <section class="panel" aria-labelledby="dom-h">
        <div class="panel-head">
          <h2 id="dom-h">{{ 'status.byDomain' | t }}</h2>
          <div class="chips" role="group" [attr.aria-label]="'status.filter' | t">
            <button type="button" class="chip" [attr.aria-pressed]="!only()" (click)="only.set('')">{{ 'features.all' | t }}</button>
            @for (st of statuses; track st.status) {
              <button type="button" class="chip" [attr.data-status]="st.status" [attr.aria-pressed]="only() === st.status" (click)="only.set(st.status)">
                <span class="status-dot" aria-hidden="true"></span> {{ st.label | t }}
              </button>
            }
          </div>
        </div>
        <div class="table-wrap">
          <table>
            <caption>{{ 'status.rows' | t: { n: rows().length } }}</caption>
            <thead>
              <tr>
                <th scope="col">{{ 'features.filterDomain' | t }}</th>
                @for (st of statuses; track st.status) {
                  <th scope="col" class="num">{{ st.label | t }}</th>
                }
                <th scope="col">{{ 'status.mix' | t }}</th>
              </tr>
            </thead>
            <tbody>
              @for (d of rows(); track d.domain) {
                <tr>
                  <th scope="row">
                    <a [routerLink]="['/features']" [queryParams]="{ domain: d.domain }" class="with-icon"><apsi-icon [name]="icon(d.domain)" /> {{ d.name }}</a>
                  </th>
                  @for (st of statuses; track st.status) {
                    <td class="num">{{ d.counts[st.status] ?? 0 }}</td>
                  }
                  <td>
                    <span class="stacked" aria-hidden="true">
                      @for (st of statuses; track st.status) {
                        <span [attr.data-status]="st.status" [style.flex-grow]="d.counts[st.status] ?? 0"></span>
                      }
                    </span>
                  </td>
                </tr>
              }
            </tbody>
          </table>
        </div>
      </section>

      <aside class="note" role="note">
        <apsi-icon name="shield" />
        <p>{{ 'status.honesty' | t }}</p>
      </aside>
    }
  `,
})
export class StatusPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  protected readonly statuses = STATUSES;
  readonly state = loadable(() => this.api.status());
  readonly only = signal<FeatureStatus | ''>('');

  readonly slices = computed<DonutSlice[]>(() => {
    const s = this.state.data();
    return STATUSES.map((st) => ({
      label: this.i18n.t(st.label),
      value: s?.by_status.find((x) => x.status === st.status)?.features ?? 0,
      tone: st.tone,
    }));
  });

  readonly testedPct = computed(() => {
    const s = this.state.data();
    if (!s) return '—';
    return this.pct(s.by_status.find((x) => x.status === 'implemented-and-tested')?.features ?? 0, s.total);
  });

  readonly groups = computed(() => {
    const byDomain = new Map((this.state.data()?.domains ?? []).map((d) => [d.domain, d.counts]));
    return GROUPS.map((g) => {
      const counts = Object.fromEntries(STATUSES.map((st) => [st.status, 0])) as Record<FeatureStatus, number>;
      for (const d of g.domains) {
        const c = byDomain.get(d);
        if (c) for (const st of STATUSES) counts[st.status] += c[st.status] ?? 0;
      }
      return { ...g, counts, total: Object.values(counts).reduce((a, n) => a + n, 0) };
    }).filter((g) => g.total > 0);
  });

  readonly rows = computed(() => {
    const only = this.only();
    return (this.state.data()?.domains ?? []).filter((d) => !only || (d.counts[only] ?? 0) > 0);
  });

  pct(n: number, total: number): string {
    return total ? `${this.i18n.number((n / total) * 100, 1)}%` : '—';
  }

  icon(domain: string) {
    return iconOf(domain);
  }

  tone(domain: string) {
    return groupOf(domain).tone;
  }

  describe(counts: Record<FeatureStatus, number>): string {
    return STATUSES.map((st) => `${this.i18n.t(st.label)}: ${counts[st.status]}`).join(', ');
  }
}
