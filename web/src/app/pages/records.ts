import { Component, effect, inject, input, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { Auth } from '../core/auth';
import { I18n, TPipe } from '../core/i18n';
import { Announcer, loadable } from '../core/load';
import { Lifecycle } from '../core/models';
import { StateView } from '../shared/state-view';

interface Issue {
  rule: string;
  field?: string;
  message: string;
  severity: string;
}

@Component({
  selector: 'apsi-records',
  imports: [FormsModule, RouterLink, StateView, TPipe],
  template: `
    <nav aria-label="breadcrumb" class="muted">
      <a [routerLink]="['/domains', domain()]">{{ domain() }}</a> / {{ entity() }}
    </nav>
    <h1 tabindex="-1">{{ entity() }}</h1>

    <form class="filters" role="search" (ngSubmit)="search()">
      <label for="q">{{ 'records.search' | t }}</label>
      <input id="q" name="q" type="search" [(ngModel)]="q" />
      <label for="lifecycle">{{ 'records.lifecycle' | t }}</label>
      <select id="lifecycle" name="lifecycle" [(ngModel)]="lifecycle">
        <option value="">{{ 'records.any' | t }}</option>
        <option value="draft">draft</option>
        <option value="active">active</option>
        <option value="archived">archived</option>
      </select>
      <label><input type="checkbox" name="archived" [(ngModel)]="includeArchived" /> {{ 'records.includeArchived' | t }}</label>
      <button type="submit">{{ 'records.search' | t }}</button>
    </form>

    <apsi-state [status]="list.status()" [error]="list.error()" (retry)="list.reload()" />
    @if (list.status() === 'ready' && list.data(); as page) {
      <table>
        <caption>{{ 'records.range' | t: { from: page.offset + 1, to: page.offset + page.items.length, total: page.total } }}</caption>
        <thead>
          <tr>
            <th scope="col">{{ 'records.name' | t }}</th>
            <th scope="col">{{ 'records.lifecycle' | t }}</th>
            <th scope="col">{{ 'records.version' | t }}</th>
            <th scope="col">{{ 'records.tags' | t }}</th>
            <th scope="col">{{ 'records.updated' | t }}</th>
          </tr>
        </thead>
        <tbody>
          @for (r of page.items; track r.id) {
            <tr>
              <th scope="row"><a [routerLink]="['/records', r.id]">{{ r.name }}</a></th>
              <td>{{ r.lifecycle }}</td>
              <td>{{ r.version }}</td>
              <td>{{ r.tags.join(', ') }}</td>
              <td><time [attr.datetime]="r.updated_at">{{ i18n.date(r.updated_at) }}</time></td>
            </tr>
          }
        </tbody>
      </table>
      <div class="pager">
        <button type="button" [disabled]="offset() === 0" (click)="turn(-1)">{{ 'records.previous' | t }}</button>
        <button type="button" [disabled]="page.next_offset === undefined || page.next_offset === null" (click)="turn(1)">{{ 'records.next' | t }}</button>
      </div>
    }

    <section aria-labelledby="export-h">
      <h2 id="export-h">{{ 'records.export' | t }}</h2>
      <button type="button" (click)="download('csv')">CSV</button>
      <button type="button" (click)="download('json')">JSON</button>
      <button type="button" (click)="download('geojson')">GeoJSON</button>
    </section>

    @if (auth.has('editor')) {
      <section aria-labelledby="new-h">
        <h2 id="new-h">{{ 'records.new' | t }}</h2>
        <form (ngSubmit)="create()" class="stack">
          <label for="new-name">{{ 'records.name' | t }}</label>
          <input id="new-name" name="name" required maxlength="256" [(ngModel)]="draft.name" />
          <label for="new-value">{{ 'records.value' | t }}</label>
          <input id="new-value" name="value" type="number" step="any" [(ngModel)]="draft.value" />
          <label for="new-unit">{{ 'records.unit' | t }}</label>
          <input id="new-unit" name="unit" maxlength="32" [(ngModel)]="draft.unit" />
          <label for="new-desc">{{ 'records.description' | t }}</label>
          <textarea id="new-desc" name="description" maxlength="4000" [(ngModel)]="draft.description"></textarea>
          <label for="new-tags">{{ 'records.tags' | t }}</label>
          <input id="new-tags" name="tags" [(ngModel)]="draft.tags" placeholder="a, b" />
          <button type="submit" [disabled]="!draft.name.trim()">{{ 'records.create' | t }}</button>
        </form>
        @if (createError(); as e) {
          <div class="state state-error" role="alert">
            <p><code>{{ e.code }}</code> {{ e.message }}</p>
            <ul>
              @for (i of issues(); track i.rule + (i.field ?? '')) {
                <li>[{{ i.severity }}] {{ i.field }}: {{ i.message }}</li>
              }
            </ul>
          </div>
        }
      </section>
    }

    <section aria-labelledby="quality-h">
      <h2 id="quality-h">{{ 'records.quality' | t }}</h2>
      <apsi-state [status]="quality.status()" [error]="quality.error()" (retry)="quality.reload()" />
      @if (quality.data(); as qr) {
        <dl class="kv">
          <dt>completeness</dt><dd>{{ pct(qr.completeness) }}</dd>
          <dt>validity</dt><dd>{{ pct(qr.validity) }}</dd>
          <dt>provenance_coverage</dt><dd>{{ pct(qr.provenance_coverage) }}</dd>
          <dt>mean_freshness_days</dt><dd>{{ qr.mean_freshness_days === null ? '—' : i18n.number(qr.mean_freshness_days, 1) }}</dd>
        </dl>
        <details>
          <summary>formulas</summary>
          <ul>
            @for (f of formulaList(qr.formulas); track f[0]) {
              <li><strong>{{ f[0] }}</strong>: {{ f[1] }}</li>
            }
          </ul>
          <p class="muted">{{ qr.missing_data }}</p>
          <p class="muted">{{ qr.limitations }}</p>
        </details>
      }
    </section>
  `,
})
export class RecordsPage {
  private readonly api = inject(Api);
  private readonly announcer = inject(Announcer);
  protected readonly auth = inject(Auth);
  protected readonly i18n = inject(I18n);
  readonly domain = input.required<string>();
  readonly entity = input.required<string>();
  readonly offset = signal(0);
  q = '';
  lifecycle: Lifecycle | '' = '';
  includeArchived = false;
  draft = { name: '', value: null as number | null, unit: '', description: '', tags: '' };
  readonly createError = signal<ApiFailure | null>(null);
  readonly issues = signal<Issue[]>([]);

  readonly list = loadable(
    () => this.api.records(this.domain(), this.entity(), { q: this.q, lifecycle: this.lifecycle, includeArchived: this.includeArchived, offset: this.offset(), limit: 25 }),
    (p) => p.items.length === 0,
    false,
  );
  readonly quality = loadable(() => this.api.quality(this.domain(), this.entity()), undefined, false);

  constructor() {
    effect(() => {
      this.domain();
      this.entity();
      untracked(() => {
        this.offset.set(0);
        this.list.reload();
        this.quality.reload();
      });
    });
  }

  search(): void {
    this.offset.set(0);
    this.list.reload();
  }

  turn(dir: number): void {
    this.offset.update((o) => Math.max(0, o + dir * 25));
    this.list.reload();
  }

  create(): void {
    const attributes: Record<string, unknown> = {};
    if (this.draft.value !== null && this.draft.value !== undefined && `${this.draft.value}` !== '') attributes['value'] = Number(this.draft.value);
    if (this.draft.unit.trim()) attributes['unit'] = this.draft.unit.trim();
    if (this.draft.description.trim()) attributes['description'] = this.draft.description.trim();
    const tags = this.draft.tags.split(',').map((t) => t.trim()).filter(Boolean);
    this.createError.set(null);
    this.issues.set([]);
    this.api.create(this.domain(), this.entity(), { name: this.draft.name.trim(), attributes, tags }).subscribe({
      next: () => {
        this.draft = { name: '', value: null, unit: '', description: '', tags: '' };
        this.announcer.say(this.i18n.t('records.created'));
        this.list.reload();
        this.quality.reload();
      },
      error: (e: unknown) => {
        const f = ApiFailure.from(e);
        this.createError.set(f);
        const details = f.details as { issues?: Issue[] } | undefined;
        this.issues.set(details?.issues ?? []);
      },
    });
  }

  download(format: 'csv' | 'json' | 'geojson'): void {
    this.api.exportFile(this.domain(), this.entity(), format).subscribe({
      next: (blob) => {
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${this.domain()}-${this.entity()}.${format}`;
        a.click();
        URL.revokeObjectURL(url);
      },
      error: (e: unknown) => this.announcer.say(ApiFailure.from(e).message),
    });
  }

  pct(v: number | null): string {
    return v === null ? '—' : `${this.i18n.number(v * 100, 1)}%`;
  }

  formulaList(f: Record<string, string>): [string, string][] {
    return Object.entries(f);
  }
}
