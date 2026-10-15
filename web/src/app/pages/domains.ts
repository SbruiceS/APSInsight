import { Component, computed, effect, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { CatalogStore } from '../core/catalog-store';
import { I18n, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { GROUPS, groupOf, iconOf } from '../core/taxonomy';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';

/** All 100 domains, grouped by agricultural area, with search and record counts. */
@Component({
  selector: 'apsi-domains',
  imports: [RouterLink, StateView, TPipe, Icon],
  template: `
    <header class="page-head">
      <div>
        <p class="eyebrow"><apsi-icon name="field" /> {{ 'nav.workspace' | t }}</p>
        <h1 tabindex="-1">{{ 'domains.title' | t }}</h1>
        <p class="muted">{{ 'domains.lead' | t }}</p>
      </div>
    </header>

    <div class="toolbar" role="search">
      <label for="dom-q" class="visually-hidden">{{ 'domains.search' | t }}</label>
      <span class="input-icon">
        <apsi-icon name="search" />
        <input id="dom-q" type="search" [placeholder]="'domains.search' | t" [value]="q()" (input)="setQ($any($event.target).value)" />
      </span>
    </div>
    <div class="chips" role="group" [attr.aria-label]="'domains.groups' | t">
      <button type="button" class="chip" [attr.aria-pressed]="!active()" (click)="setGroup('')">
        <apsi-icon name="layers" /> {{ 'features.all' | t }}
      </button>
      @for (g of groups; track g.id) {
        <button type="button" class="chip" [attr.data-tone]="g.tone" [attr.aria-pressed]="active() === g.id" (click)="setGroup(g.id)">
          <apsi-icon [name]="g.icon" /> {{ g.label | t }}
        </button>
      }
    </div>

    @if (!store.catalog()) {
      <apsi-state [status]="store.failed() ? 'error' : 'loading'" (retry)="store.ensure()" />
    } @else {
      <p class="muted" role="status">{{ 'domains.count' | t: { n: cards().length } }}</p>
      @if (cards().length === 0) {
        <div class="empty-hint"><apsi-icon name="seed" [live]="true" size="xl" /><p>{{ 'domains.none' | t }}</p></div>
      }
      <ul class="domain-grid" role="list">
        @for (d of cards(); track d.slug) {
          <li class="domain-card" [attr.data-tone]="d.tone">
            <a class="domain-card-main" [routerLink]="['/domains', d.slug]">
              <span class="domain-icon"><apsi-icon [name]="d.icon" size="lg" /></span>
              <span class="domain-title">{{ d.name }}</span>
              <span class="domain-meta">{{ d.groupLabel | t }} · {{ 'domains.kinds' | t: { n: d.kinds } }}</span>
              <span class="domain-count"><strong>{{ i18n.number(d.records) }}</strong> {{ 'overview.records' | t }}</span>
            </a>
            <ul class="mini-links" role="list" [attr.aria-label]="'domain.entities' | t">
              @for (e of d.entities; track e.slug) {
                <li><a [routerLink]="['/domains', d.slug, e.slug]">{{ e.name }}</a></li>
              }
            </ul>
          </li>
        }
      </ul>
    }
  `,
})
export class DomainsPage {
  private readonly api = inject(Api);
  private readonly router = inject(Router);
  protected readonly store = inject(CatalogStore);
  protected readonly i18n = inject(I18n);
  protected readonly groups = GROUPS;
  /** Bound from `?group=` and `?q=`, so filtered views can be linked and bookmarked. */
  readonly group = input<string | undefined>(undefined);
  readonly query = input<string | undefined>(undefined, { alias: 'q' });
  readonly active = signal('');
  readonly q = signal('');
  readonly counts = loadable(() => this.api.overview());

  constructor() {
    this.store.ensure();
    effect(() => this.active.set(this.group() ?? ''));
    effect(() => this.q.set(this.query() ?? ''));
  }

  readonly cards = computed(() => {
    const records = new Map((this.counts.data()?.domains ?? []).map((d) => [d.domain, d.records]));
    const terms = this.q().toLowerCase().trim().split(/\s+/).filter(Boolean);
    return (this.store.catalog()?.domains ?? [])
      .filter((d) => !this.active() || groupOf(d.slug).id === this.active())
      .filter((d) => {
        const hay = (d.name + ' ' + d.slug + ' ' + d.entities.map((e) => e.name).join(' ')).toLowerCase();
        return terms.every((t) => hay.includes(t));
      })
      .map((d) => {
        const g = groupOf(d.slug);
        return {
          slug: d.slug,
          name: d.name,
          kinds: d.entities.length,
          entities: d.entities.slice(0, 4),
          records: records.get(d.slug) ?? 0,
          icon: iconOf(d.slug),
          tone: g.tone,
          groupLabel: g.label,
        };
      });
  });

  setGroup(id: string): void {
    this.active.set(id);
    this.sync();
  }

  setQ(v: string): void {
    this.q.set(v);
    this.sync();
  }

  private sync(): void {
    void this.router.navigate([], { queryParams: { group: this.active() || null, q: this.q() || null }, replaceUrl: true });
  }
}
