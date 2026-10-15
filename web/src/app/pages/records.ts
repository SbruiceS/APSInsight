import { Component, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { Auth } from '../core/auth';
import { CatalogStore } from '../core/catalog-store';
import { I18n, TPipe } from '../core/i18n';
import { Announcer, loadable } from '../core/load';
import { Lifecycle } from '../core/models';
import { groupOf, iconOf } from '../core/taxonomy';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';
import { Meter } from '../shared/widgets';

interface Issue {
  rule: string;
  field?: string;
  message: string;
  severity: string;
}

interface Describe {
  n: number;
  mean: number;
  sd: number | null;
  min: number;
  median: number;
  max: number;
}

/** Common unit codes from the units catalogue (docs/UNITS.md); suggestions only. */
export const COMMON_UNITS = ['1', '%', 'kg/ha', 't/ha', 'lb/ac', 'mm', 'mm/d', 'Cel', 'K', 'm3/m3', 'g/kg', 'mg/kg', 'dS/m', 'cmol/kg', 'g/cm3', 'kPa', 'W/m2', 'MJ/m2/d', 'm/s', 'ha', 'm', 'kg', 't'];

@Component({
  selector: 'apsi-records',
  imports: [FormsModule, RouterLink, StateView, TPipe, Icon, Meter],
  template: `
    <nav aria-label="breadcrumb" class="breadcrumb">
      <a routerLink="/domains">{{ 'nav.domains' | t }}</a>
      <span aria-hidden="true">›</span>
      <a [routerLink]="['/domains', domain()]">{{ store.domainName(domain()) }}</a>
      <span aria-hidden="true">›</span>
      <span>{{ store.entityName(domain(), entity()) }}</span>
    </nav>
    <header class="page-head" [attr.data-tone]="tone()">
      <span class="page-icon"><apsi-icon [name]="icon()" [live]="true" size="xl" /></span>
      <div>
        <h1 tabindex="-1">{{ store.entityName(domain(), entity()) }}</h1>
        <p class="muted"><code>{{ domain() }}/{{ entity() }}</code></p>
      </div>
      <div class="page-actions">
        <span class="export-group" role="group" [attr.aria-label]="'records.export' | t">
          <apsi-icon name="download" />
          <button type="button" class="ghost" (click)="download('csv')">CSV</button>
          <button type="button" class="ghost" (click)="download('json')">JSON</button>
          <button type="button" class="ghost" (click)="download('geojson')">GeoJSON</button>
        </span>
      </div>
    </header>

    <form class="toolbar" role="search" (ngSubmit)="search()">
      <label for="q">{{ 'records.search' | t }}</label>
      <span class="input-icon"><apsi-icon name="search" /><input id="q" name="q" type="search" [(ngModel)]="q" /></span>
      <label for="lifecycle">{{ 'records.lifecycle' | t }}</label>
      <select id="lifecycle" name="lifecycle" [(ngModel)]="lifecycle">
        <option value="">{{ 'records.any' | t }}</option>
        <option value="draft">draft</option>
        <option value="active">active</option>
        <option value="archived">archived</option>
      </select>
      <label class="check"><input type="checkbox" name="archived" [(ngModel)]="includeArchived" /> {{ 'records.includeArchived' | t }}</label>
      <button type="submit"><apsi-icon name="filter" /> {{ 'records.search' | t }}</button>
    </form>

    <div class="two-col wide-left">
      <section class="panel" aria-labelledby="list-h">
        <h2 id="list-h" class="visually-hidden">{{ 'overview.records' | t }}</h2>
        <apsi-state [status]="list.status()" [error]="list.error()" (retry)="list.reload()" />
        @if (list.status() === 'ready' && list.data(); as page) {
          <div class="table-wrap">
            <table>
              <caption>{{ 'records.range' | t: { from: page.offset + 1, to: page.offset + page.items.length, total: page.total } }}</caption>
              <thead>
                <tr>
                  <th scope="col">{{ 'records.name' | t }}</th>
                  <th scope="col">{{ 'records.value' | t }}</th>
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
                    <td>{{ show(r.attributes['value']) }} <span class="muted">{{ unit(r.attributes['unit']) }}</span></td>
                    <td><span class="badge" [attr.data-lifecycle]="r.lifecycle">{{ r.lifecycle }}</span></td>
                    <td>v{{ r.version }}</td>
                    <td>
                      @for (t of r.tags; track t) {
                        <span class="tag">{{ t }}</span>
                      }
                    </td>
                    <td><time [attr.datetime]="r.updated_at">{{ i18n.date(r.updated_at) }}</time></td>
                  </tr>
                }
              </tbody>
            </table>
          </div>
          <div class="pager">
            <button type="button" class="ghost" [disabled]="offset() === 0" (click)="turn(-1)">{{ 'records.previous' | t }}</button>
            <button type="button" class="ghost" [disabled]="page.next_offset === undefined || page.next_offset === null" (click)="turn(1)">{{ 'records.next' | t }}</button>
          </div>

          <section class="analysis" aria-labelledby="an-h">
            <h3 id="an-h"><apsi-icon name="chart" /> {{ 'records.analyse' | t }}</h3>
            <p class="muted small">{{ 'records.analyseHint' | t: { n: values().length } }}</p>
            <div class="actions">
              <button type="button" [disabled]="values().length === 0" (click)="describe()"><apsi-icon name="chart" /> {{ 'records.describe' | t }}</button>
              @if (values().length > 0) {
                <a class="button-link ghost" routerLink="/lab" [queryParams]="labLink()"><apsi-icon name="flask" /> {{ 'records.openLab' | t }}</a>
              }
              <a class="button-link ghost" routerLink="/engine" [queryParams]="{ table: sqlTable() }"><apsi-icon name="database" /> {{ 'records.querySql' | t }}</a>
            </div>
            @if (stats(); as s) {
              <dl class="stat-strip">
                <div><dt>n</dt><dd>{{ s.n }}</dd></div>
                <div><dt>{{ 'records.mean' | t }}</dt><dd>{{ i18n.number(s.mean, 3) }}</dd></div>
                <div><dt>{{ 'records.sd' | t }}</dt><dd>{{ s.sd === null ? '—' : i18n.number(s.sd, 3) }}</dd></div>
                <div><dt>{{ 'records.min' | t }}</dt><dd>{{ i18n.number(s.min, 3) }}</dd></div>
                <div><dt>{{ 'records.median' | t }}</dt><dd>{{ i18n.number(s.median, 3) }}</dd></div>
                <div><dt>{{ 'records.max' | t }}</dt><dd>{{ i18n.number(s.max, 3) }}</dd></div>
              </dl>
            }
          </section>
        }
      </section>

      <aside class="side-stack">
        @if (auth.has('editor')) {
          <section class="panel" aria-labelledby="new-h">
            <h2 id="new-h"><apsi-icon name="sprout" /> {{ 'records.new' | t }}</h2>
            <form (ngSubmit)="create()" class="stack">
              <label for="new-name">{{ 'records.name' | t }}</label>
              <input id="new-name" name="name" required maxlength="256" [(ngModel)]="draft.name" />
              <div class="field-pair">
                <span class="stack">
                  <label for="new-value">{{ 'records.value' | t }}</label>
                  <input id="new-value" name="value" type="number" step="any" [(ngModel)]="draft.value" />
                </span>
                <span class="stack">
                  <label for="new-unit">{{ 'records.unit' | t }}</label>
                  <input id="new-unit" name="unit" maxlength="32" list="unit-options" [(ngModel)]="draft.unit" aria-describedby="unit-help" />
                </span>
              </div>
              <datalist id="unit-options">
                @for (u of units; track u) {
                  <option [value]="u"></option>
                }
              </datalist>
              <p id="unit-help" class="muted small">{{ 'records.unitHelp' | t }}</p>
              <label for="new-desc">{{ 'records.description' | t }}</label>
              <textarea id="new-desc" name="description" maxlength="4000" [(ngModel)]="draft.description"></textarea>
              <label for="new-tags">{{ 'records.tags' | t }}</label>
              <input id="new-tags" name="tags" [(ngModel)]="draft.tags" placeholder="north, irrigated" />
              <button type="submit" class="primary" [disabled]="!draft.name.trim()"><apsi-icon name="plus" /> {{ 'records.create' | t }}</button>
            </form>
            @if (createError(); as e) {
              <div class="state state-error" role="alert">
                <apsi-icon name="alert" />
                <div>
                  <p><code>{{ e.code }}</code> {{ e.message }}</p>
                  <ul>
                    @for (i of issues(); track i.rule + (i.field ?? '')) {
                      <li>[{{ i.severity }}] {{ i.field }}: {{ i.message }}</li>
                    }
                  </ul>
                </div>
              </div>
            }
          </section>
        }

        <section class="panel" aria-labelledby="quality-h">
          <h2 id="quality-h"><apsi-icon name="leaf" /> {{ 'records.quality' | t }}</h2>
          <apsi-state [status]="quality.status()" [error]="quality.error()" (retry)="quality.reload()" />
          @if (quality.data(); as qr) {
            <apsi-meter [label]="'domain.completeness' | t" [value]="qr.completeness" />
            <apsi-meter [label]="'domain.validity' | t" [value]="qr.validity" />
            <apsi-meter [label]="'records.provenance' | t" [value]="qr.provenance_coverage" />
            <p class="small">{{ 'records.freshness' | t }}: <strong>{{ qr.mean_freshness_days === null ? '—' : i18n.number(qr.mean_freshness_days, 1) }}</strong></p>
            <details>
              <summary>{{ 'records.formulas' | t }}</summary>
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
      </aside>
    </div>
  `,
})
export class RecordsPage {
  private readonly api = inject(Api);
  private readonly announcer = inject(Announcer);
  protected readonly auth = inject(Auth);
  protected readonly i18n = inject(I18n);
  protected readonly store = inject(CatalogStore);
  protected readonly units = COMMON_UNITS;
  readonly domain = input.required<string>();
  readonly entity = input.required<string>();
  readonly offset = signal(0);
  q = '';
  lifecycle: Lifecycle | '' = '';
  includeArchived = false;
  draft = { name: '', value: null as number | null, unit: '', description: '', tags: '' };
  readonly createError = signal<ApiFailure | null>(null);
  readonly issues = signal<Issue[]>([]);
  readonly stats = signal<Describe | null>(null);
  readonly icon = computed(() => iconOf(this.domain()));
  readonly tone = computed(() => groupOf(this.domain()).tone);

  readonly list = loadable(
    () => this.api.records(this.domain(), this.entity(), { q: this.q, lifecycle: this.lifecycle, includeArchived: this.includeArchived, offset: this.offset(), limit: 25 }),
    (p) => p.items.length === 0,
    false,
  );
  readonly quality = loadable(() => this.api.quality(this.domain(), this.entity()), undefined, false);

  /** Numeric `value` attributes on the current page, for quick statistics. */
  readonly values = computed(() =>
    (this.list.data()?.items ?? []).map((r) => r.attributes['value']).filter((v): v is number => typeof v === 'number' && Number.isFinite(v)),
  );
  /** SQL table name used by the Compute Engine (`domain__entity`, dashes as underscores). */
  readonly sqlTable = computed(() => this.domain().replace(/-/g, '_') + '__' + this.entity().replace(/-/g, '_'));
  readonly labLink = computed(() => ({ fn: 'stats.describe', input: JSON.stringify(this.values()) }));

  constructor() {
    effect(() => {
      this.domain();
      this.entity();
      untracked(() => {
        this.offset.set(0);
        this.stats.set(null);
        this.list.reload();
        this.quality.reload();
      });
    });
  }

  search(): void {
    this.offset.set(0);
    this.stats.set(null);
    this.list.reload();
  }

  turn(dir: number): void {
    this.offset.update((o) => Math.max(0, o + dir * 25));
    this.stats.set(null);
    this.list.reload();
  }

  show(v: unknown, empty = '—'): string {
    if (v === undefined || v === null || v === '') return empty;
    if (typeof v === 'number') return this.i18n.number(v, 4);
    return typeof v === 'object' ? JSON.stringify(v) : String(v as string | boolean);
  }

  /** Dimensionless values (unit `1`) show no unit suffix. */
  unit(u: unknown): string {
    return typeof u === 'string' && u !== '1' ? u : '';
  }

  describe(): void {
    this.api.compute('stats.describe', this.values()).subscribe({
      next: (r) => this.stats.set(r as Describe),
      error: (e: unknown) => this.announcer.say(ApiFailure.from(e).message, 'error'),
    });
  }

  create(): void {
    const attributes: Record<string, unknown> = {};
    if (this.draft.value !== null && this.draft.value !== undefined && `${this.draft.value}` !== '') attributes['value'] = Number(this.draft.value);
    if (this.draft.unit.trim()) attributes['unit'] = this.draft.unit.trim();
    if (this.draft.description.trim()) attributes['description'] = this.draft.description.trim();
    const tags = this.draft.tags
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean);
    this.createError.set(null);
    this.issues.set([]);
    this.api.create(this.domain(), this.entity(), { name: this.draft.name.trim(), attributes, tags }).subscribe({
      next: () => {
        this.draft = { name: '', value: null, unit: '', description: '', tags: '' };
        this.announcer.say(this.i18n.t('records.created'), 'success');
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
        this.announcer.say(this.i18n.t('records.exported', { format: format.toUpperCase() }), 'success');
      },
      error: (e: unknown) => this.announcer.say(ApiFailure.from(e).message, 'error'),
    });
  }

  formulaList(f: Record<string, string>): [string, string][] {
    return Object.entries(f);
  }
}
