import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Api, ApiFailure } from '../core/api.service';
import { I18nKey, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { Feature, FeatureStatus, TraceRow } from '../core/models';
import { StateView } from '../shared/state-view';

@Component({
  selector: 'apsi-features',
  imports: [FormsModule, StateView, TPipe],
  template: `
    <h1 tabindex="-1">{{ 'features.title' | t }}</h1>
    @if (catalog.data(); as c) {
      <p class="muted">{{ c.features }} features · {{ c.domains.length }} domains · {{ c.capabilities.length }} capabilities · {{ c.entity_kinds }} entity kinds</p>
      <form class="filters" role="search" (ngSubmit)="apply()">
        <label for="f-domain">{{ 'features.filterDomain' | t }}</label>
        <select id="f-domain" name="domain" [(ngModel)]="domain">
          <option value="">{{ 'features.all' | t }}</option>
          @for (d of c.domains; track d.slug) {
            <option [value]="d.slug">{{ d.name }}</option>
          }
        </select>
        <label for="f-cap">{{ 'features.filterCapability' | t }}</label>
        <select id="f-cap" name="capability" [(ngModel)]="capability">
          <option value="">{{ 'features.all' | t }}</option>
          @for (k of c.capabilities; track k.slug) {
            <option [value]="k.slug">{{ k.title }}</option>
          }
        </select>
        <label for="f-q">{{ 'features.query' | t }}</label>
        <input id="f-q" name="q" type="search" [(ngModel)]="q" />
        <button type="submit">{{ 'records.search' | t }}</button>
      </form>
    }
    <apsi-state [status]="list.status()" [error]="list.error()" (retry)="list.reload()" />
    @if (list.status() === 'ready' && list.data(); as page) {
      <table>
        <caption>{{ 'records.range' | t: { from: page.offset + 1, to: page.offset + page.items.length, total: page.total } }}</caption>
        <thead>
          <tr><th scope="col">ID</th><th scope="col">{{ 'features.filterDomain' | t }}</th><th scope="col">{{ 'features.filterCapability' | t }}</th><th scope="col">{{ 'features.requirement' | t }}</th></tr>
        </thead>
        <tbody>
          @for (f of page.items; track f.id) {
            <tr>
              <th scope="row"><button type="button" class="link" (click)="select(f)" [attr.aria-expanded]="selected()?.feature_id === f.id">{{ f.id }}</button></th>
              <td>{{ f.domain }}</td>
              <td>{{ f.title }}</td>
              <td>{{ f.requirement }}</td>
            </tr>
            @if (selected()?.feature_id === f.id) {
              <tr>
                <td colspan="4">
                  @if (selected(); as ev) {
                    <h2 class="h3">{{ 'features.evidence' | t }}</h2>
                    <dl class="kv">
                      <dt>{{ 'features.status' | t }}</dt><dd><span class="badge" [attr.data-status]="ev.status">{{ statusKey[ev.status] | t }}</span></dd>
                      <dt>{{ 'features.gap' | t }}</dt><dd>{{ ev.gap || '—' }}</dd>
                      <dt>{{ 'features.engine' | t }}</dt><dd>{{ ev.engine.join('; ') }}</dd>
                      <dt>{{ 'features.api' | t }}</dt><dd>{{ ev.api.join('; ') || '—' }}</dd>
                      <dt>CLI</dt><dd>{{ ev.cli.join('; ') || '—' }}</dd>
                      <dt>UI</dt><dd>{{ ev.ui }}</dd>
                      <dt>{{ 'features.tests' | t }}</dt><dd>{{ ev.tests.join('; ') }}</dd>
                      <dt>Docs</dt><dd>{{ ev.docs }}</dd>
                    </dl>
                  }
                </td>
              </tr>
            }
          }
        </tbody>
      </table>
      <div class="pager">
        <button type="button" [disabled]="offset === 0" (click)="turn(-1)">{{ 'records.previous' | t }}</button>
        <button type="button" [disabled]="offset + page.items.length >= page.total" (click)="turn(1)">{{ 'records.next' | t }}</button>
      </div>
    }
    @if (selectError()) {
      <p role="alert" class="state state-error">{{ selectError() }}</p>
    }
  `,
})
export class FeaturesPage {
  private readonly api = inject(Api);
  readonly catalog = loadable(() => this.api.catalog());
  domain = '';
  capability = '';
  q = '';
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
    () => this.api.features({ domain: this.domain, capability: this.capability, q: this.q, offset: this.offset, limit: 50 }),
    (p) => p.items.length === 0,
  );

  apply(): void {
    this.offset = 0;
    this.list.reload();
  }

  turn(dir: number): void {
    this.offset = Math.max(0, this.offset + dir * 50);
    this.list.reload();
  }

  select(f: Feature): void {
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
