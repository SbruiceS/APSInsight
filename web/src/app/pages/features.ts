import { Component, effect, inject, input, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterLink } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { CatalogStore } from '../core/catalog-store';
import { I18nKey, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { Feature, FeatureStatus, TraceRow } from '../core/models';
import { iconOf } from '../core/taxonomy';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';

@Component({
  selector: 'apsi-features',
  imports: [FormsModule, RouterLink, StateView, TPipe, Icon],
  template: `
    <header class="page-head" data-tone="sky">
      <span class="page-icon"><apsi-icon name="book" [live]="true" size="xl" /></span>
      <div>
        <p class="eyebrow">{{ 'nav.governance' | t }}</p>
        <h1 tabindex="-1">{{ 'features.title' | t }}</h1>
        @if (store.catalog(); as c) {
          <p class="muted">{{ 'features.summary' | t: { features: c.features, domains: c.domains.length, capabilities: c.capabilities.length, kinds: c.entity_kinds } }}</p>
        }
      </div>
      <div class="page-actions">
        <a class="button-link ghost" routerLink="/status"><apsi-icon name="gauge" /> {{ 'nav.status' | t }}</a>
      </div>
    </header>

    @if (store.catalog(); as c) {
      <form class="toolbar" role="search" (ngSubmit)="apply()">
        <label for="f-domain">{{ 'features.filterDomain' | t }}</label>
        <select id="f-domain" name="domain" [(ngModel)]="fDomain">
          <option value="">{{ 'features.all' | t }}</option>
          @for (d of c.domains; track d.slug) {
            <option [value]="d.slug">{{ d.name }}</option>
          }
        </select>
        <label for="f-cap">{{ 'features.filterCapability' | t }}</label>
        <select id="f-cap" name="capability" [(ngModel)]="fCapability">
          <option value="">{{ 'features.all' | t }}</option>
          @for (k of c.capabilities; track k.slug) {
            <option [value]="k.slug">{{ k.title }}</option>
          }
        </select>
        <label for="f-q">{{ 'features.query' | t }}</label>
        <span class="input-icon"><apsi-icon name="search" /><input id="f-q" name="q" type="search" [(ngModel)]="fQ" /></span>
        <button type="submit"><apsi-icon name="filter" /> {{ 'records.search' | t }}</button>
      </form>
    }

    <section class="panel">
      <apsi-state [status]="list.status()" [error]="list.error()" (retry)="list.reload()" />
      @if (list.status() === 'ready' && list.data(); as page) {
        <div class="table-wrap">
          <table>
            <caption>{{ 'records.range' | t: { from: page.offset + 1, to: page.offset + page.items.length, total: page.total } }}</caption>
            <thead>
              <tr>
                <th scope="col">ID</th>
                <th scope="col">{{ 'features.filterDomain' | t }}</th>
                <th scope="col">{{ 'features.filterCapability' | t }}</th>
                <th scope="col">{{ 'features.requirement' | t }}</th>
              </tr>
            </thead>
            <tbody>
              @for (f of page.items; track f.id) {
                <tr [class.selected]="selected()?.feature_id === f.id">
                  <th scope="row">
                    <button type="button" class="link" (click)="select(f)" [attr.aria-expanded]="selected()?.feature_id === f.id">{{ f.id }}</button>
                  </th>
                  <td>
                    <a [routerLink]="['/domains', slug(f)]" class="with-icon"><apsi-icon [name]="icon(slug(f))" /> {{ f.domain }}</a>
                  </td>
                  <td>{{ f.title }}</td>
                  <td>{{ f.requirement }}</td>
                </tr>
                @if (selected()?.feature_id === f.id) {
                  <tr class="evidence-row">
                    <td colspan="4">
                      @if (selected(); as ev) {
                        <h2 class="h3"><apsi-icon name="scroll" /> {{ 'features.evidence' | t }}</h2>
                        <dl class="kv">
                          <dt>{{ 'features.status' | t }}</dt>
                          <dd><span class="badge" [attr.data-status]="ev.status"><span class="status-dot" aria-hidden="true"></span> {{ statusKey[ev.status] | t }}</span></dd>
                          <dt>{{ 'features.gap' | t }}</dt><dd>{{ ev.gap || '—' }}</dd>
                          <dt>{{ 'features.engine' | t }}</dt><dd>{{ ev.engine.join('; ') }}</dd>
                          <dt>{{ 'features.api' | t }}</dt><dd>{{ ev.api.join('; ') || '—' }}</dd>
                          <dt>CLI</dt><dd>{{ ev.cli.join('; ') || '—' }}</dd>
                          <dt>UI</dt><dd>{{ ev.ui }}</dd>
                          <dt>{{ 'features.tests' | t }}</dt><dd>{{ ev.tests.join('; ') }}</dd>
                          <dt>Docs</dt><dd>{{ ev.docs }}</dd>
                        </dl>
                        @if (ev.entity_kinds.length) {
                          <p class="small">
                            {{ 'domain.entities' | t }}:
                            @for (k of ev.entity_kinds; track k) {
                              <a class="tag" [routerLink]="['/domains', ev.domain_slug, k]">{{ store.entityName(ev.domain_slug, k) }}</a>
                            }
                          </p>
                        }
                      }
                    </td>
                  </tr>
                }
              }
            </tbody>
          </table>
        </div>
        <div class="pager">
          <button type="button" class="ghost" [disabled]="offset === 0" (click)="turn(-1)">{{ 'records.previous' | t }}</button>
          <button type="button" class="ghost" [disabled]="offset + page.items.length >= page.total" (click)="turn(1)">{{ 'records.next' | t }}</button>
        </div>
      }
      @if (selectError()) {
        <p role="alert" class="state state-error"><apsi-icon name="alert" /> {{ selectError() }}</p>
      }
    </section>
  `,
})
export class FeaturesPage {
  private readonly api = inject(Api);
  private readonly router = inject(Router);
  protected readonly store = inject(CatalogStore);
  /** Bound from `?domain=` and `?q=`, so the domain, status and lab pages can deep-link here. */
  readonly domain = input<string | undefined>(undefined);
  readonly q = input<string | undefined>(undefined);
  fDomain = '';
  fCapability = '';
  fQ = '';
  offset = 0;
  /** Status is shown as text, never by colour alone. */
  readonly statusKey: Record<FeatureStatus, I18nKey> = {
    'implemented-and-tested': 'status.tested',
    'implemented-not-fully-tested': 'status.partial',
    'specified-only': 'status.specified',
    'blocked-external': 'status.blocked',
  };
  readonly selected = signal<TraceRow | null>(null);
  readonly selectError = signal('');
  readonly list = loadable(
    () => this.api.features({ domain: this.fDomain, capability: this.fCapability, q: this.fQ, offset: this.offset, limit: 50 }),
    (p) => p.items.length === 0,
    false,
  );

  constructor() {
    this.store.ensure();
    effect(() => {
      const d = this.domain() ?? '';
      const q = this.q() ?? '';
      untracked(() => {
        this.fDomain = d;
        this.fQ = q;
        this.offset = 0;
        this.selected.set(null);
        this.list.reload();
      });
    });
  }

  slug(f: Feature): string {
    return this.store.catalog()?.domains.find((d) => d.name === f.domain)?.slug ?? f.domain;
  }

  icon(slug: string) {
    return iconOf(slug);
  }

  apply(): void {
    this.offset = 0;
    if (this.fDomain === (this.domain() ?? '') && this.fQ === (this.q() ?? '')) {
      this.list.reload();
      return;
    }
    // The URL change re-binds the inputs, and the effect above reloads.
    void this.router.navigate([], { queryParams: { domain: this.fDomain || null, q: this.fQ || null }, replaceUrl: true });
  }

  turn(dir: number): void {
    this.offset = Math.max(0, this.offset + dir * 50);
    this.list.reload();
  }

  select(f: Feature): void {
    this.selectError.set('');
    if (this.selected()?.feature_id === f.id) {
      this.selected.set(null);
      return;
    }
    this.api.feature(f.id).subscribe({
      next: (r) => this.selected.set(r.evidence),
      error: (e: unknown) => this.selectError.set(ApiFailure.from(e).message),
    });
  }
}
