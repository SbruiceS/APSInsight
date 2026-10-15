import { Component, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { I18n, I18nKey, TPipe } from '../core/i18n';
import { Announcer, loadable } from '../core/load';
import { BackendKind, EngineTable, EngineTableInfo, PlanReport, RunReport, SqlResult } from '../core/models';
import { Icon } from '../shared/icon';
import { IconName } from '../shared/icons';
import { StateView } from '../shared/state-view';

type Tab = 'sql' | 'workflow';

const BACKEND: Record<BackendKind, { label: I18nKey; icon: IconName }> = {
  cpu: { label: 'engine.cpu', icon: 'chip' },
  native: { label: 'engine.native', icon: 'gauge' },
  accelerator: { label: 'engine.accelerator', icon: 'sparkle' },
};

/** Synthetic example workflows; they run on inline data, so they work on an empty tenant. */
export const ENGINE_EXAMPLES: { id: string; label: I18nKey; workflow: object }[] = [
  {
    id: 'ndvi',
    label: 'engine.exNdvi',
    workflow: {
      nodes: [
        {
          id: 'pixels',
          op: 'values',
          columns: [
            { name: 'plot', values: ['north', 'north', 'south', 'south', 'south'] },
            { name: 'red', unit: '1', values: [0.08, 0.1, 0.12, -9999, 0.2] },
            { name: 'nir', unit: '1', values: [0.42, 0.38, 0.3, 0.4, 0.21] },
          ],
        },
        { id: 'veg', op: 'kernel', input: 'pixels', kernel: 'ndvi', nodata: -9999 },
        {
          id: 'by_plot',
          op: 'aggregate',
          input: 'veg',
          group_by: ['plot'],
          aggregates: [
            { name: 'pixels', func: 'count', column: 'ndvi' },
            { name: 'mean_ndvi', func: 'mean', column: 'ndvi' },
            { name: 'sd_ndvi', func: 'sd', column: 'ndvi' },
          ],
        },
      ],
      outputs: ['by_plot', 'veg'],
      checks: [{ node: 'veg', column: 'ndvi', min: -1, max: 1 }],
    },
  },
  {
    id: 'units',
    label: 'engine.exUnits',
    workflow: {
      nodes: [
        {
          id: 'trial',
          op: 'values',
          columns: [
            { name: 'treatment', values: ['control', 'control', 'lime', 'lime'] },
            { name: 'yield', unit: 't/ha', values: [4.1, 3.9, 5.2, 5.6] },
            { name: 'rain', unit: 'mm', values: [512, 498, 505, 520] },
          ],
        },
        {
          id: 'kg',
          op: 'project',
          input: 'trial',
          columns: [
            { name: 'treatment', expr: { col: 'treatment' } },
            { name: 'yield_kg_ha', expr: { convert: [{ col: 'yield' }, 'kg/ha'] } },
            { name: 'water_use', expr: { div: [{ convert: [{ col: 'yield' }, 'kg/ha'] }, { col: 'rain' }] } },
          ],
        },
        { id: 'good', op: 'filter', input: 'kg', predicate: { gt: [{ col: 'yield_kg_ha' }, { lit: 4000 }] } },
        {
          id: 'summary',
          op: 'aggregate',
          input: 'good',
          group_by: ['treatment'],
          aggregates: [
            { name: 'mean_kg_ha', func: 'mean', column: 'yield_kg_ha' },
            { name: 'median_wue', func: 'median', column: 'water_use' },
          ],
        },
      ],
      outputs: ['summary'],
    },
  },
  {
    id: 'spectrum',
    label: 'engine.exSpectrum',
    workflow: {
      nodes: [
        {
          id: 'moisture',
          op: 'values',
          columns: [{ name: 'theta', unit: 'm3/m3', values: [0.31, 0.29, 0.27, 0.29, 0.31, 0.29, 0.27, 0.29, 0.31, 0.29, 0.27, 0.29, 0.31, 0.29, 0.27, 0.29] }],
        },
        { id: 'smooth', op: 'kernel', input: 'moisture', kernel: 'moving_mean', column: 'theta', window: 4, output: 'theta_4' },
        { id: 'spectrum', op: 'kernel', input: 'moisture', kernel: 'fft_power', column: 'theta' },
        { id: 'peaks', op: 'sort', input: 'spectrum', by: [{ column: 'power', descending: true }] },
        { id: 'top', op: 'limit', input: 'peaks', n: 3 },
      ],
      outputs: ['top', 'smooth'],
    },
  },
];

@Component({
  selector: 'apsi-engine',
  imports: [FormsModule, RouterLink, StateView, TPipe, Icon],
  template: `
    <header class="page-head" data-tone="sky">
      <span class="page-icon"><apsi-icon name="chip" [live]="true" size="xl" /></span>
      <div>
        <p class="eyebrow">{{ 'nav.science' | t }}</p>
        <h1 tabindex="-1">{{ 'engine.title' | t }}</h1>
        <p class="muted">{{ 'engine.lead' | t }}</p>
      </div>
      <div class="page-actions">
        <a class="button-link ghost" routerLink="/lab"><apsi-icon name="flask" /> {{ 'nav.lab' | t }}</a>
      </div>
    </header>

    <apsi-state [status]="caps.status()" [error]="caps.error()" (retry)="caps.reload()" />
    @if (caps.data(); as c) {
      <ul class="integration-grid" role="list" [attr.aria-label]="'engine.backends' | t">
        @for (b of c.backends; track b.kind) {
          <li class="integration-card" [attr.data-state]="b.available ? 'available' : 'unavailable'">
            <div class="entity-head">
              <span class="domain-icon"><apsi-icon [name]="backend[b.kind].icon" size="lg" [live]="b.available" /></span>
              <h2 class="h3">{{ backend[b.kind].label | t }}</h2>
            </div>
            <p class="avail">
              @if (b.available) {
                <span class="badge good"><apsi-icon name="check" /> {{ 'integrations.available' | t }}</span>
              } @else {
                <span class="badge warn"><apsi-icon name="alert" /> {{ 'integrations.unavailable' | t }}</span>
              }
            </p>
            <p class="small">{{ b.detail }}</p>
            <p>
              @for (k of b.kernels; track k) {
                <code class="tag">{{ k }}</code>
              }
            </p>
          </li>
        }
      </ul>
      <p class="muted small">{{ 'engine.stack' | t: { sql: c.sql.engine, arrow: c.sql.columnar_format } }}</p>
    }

    <div class="chips" role="group" [attr.aria-label]="'engine.mode' | t">
      <button type="button" class="chip" [attr.aria-pressed]="tab() === 'sql'" (click)="tab.set('sql')"><apsi-icon name="database" /> {{ 'engine.sql' | t }}</button>
      <button type="button" class="chip" [attr.aria-pressed]="tab() === 'workflow'" (click)="tab.set('workflow')"><apsi-icon name="network" /> {{ 'engine.workflow' | t }}</button>
    </div>

    @if (tab() === 'sql') {
      <div class="lab">
        <aside class="panel lab-list" aria-labelledby="tables-h">
          <h2 id="tables-h"><apsi-icon name="layers" /> {{ 'engine.tables' | t }}</h2>
          <apsi-state [status]="tables.status()" [error]="tables.error()" (retry)="tables.reload()" />
          @if (tables.status() === 'empty') {
            <p class="muted small">{{ 'engine.noTables' | t }}</p>
            <a routerLink="/domains" class="small">{{ 'overview.qaDomains' | t }} <apsi-icon name="arrowRight" /></a>
          }
          <ul role="list" class="fn-list">
            @for (t of tables.data() ?? []; track t.table) {
              <li>
                <button type="button" class="fn-button" (click)="useTable(t)">
                  <code>{{ t.table }}</code>
                  <span class="muted small">{{ 'engine.tableRows' | t: { n: t.rows, c: t.columns.length } }}</span>
                </button>
              </li>
            }
          </ul>
        </aside>

        <div class="side-stack">
          <section class="panel" aria-labelledby="sql-h">
            <h2 id="sql-h"><apsi-icon name="database" /> {{ 'engine.sql' | t }}</h2>
            <label for="sql-text">{{ 'engine.query' | t }}</label>
            <textarea id="sql-text" class="code" rows="7" spellcheck="false" [ngModel]="sql()" (ngModelChange)="sql.set($event)" aria-describedby="sql-help"></textarea>
            <p id="sql-help" class="muted small">{{ 'engine.sqlHelp' | t }}</p>
            <div class="toolbar">
              <label for="sql-max">{{ 'engine.maxRows' | t }}</label>
              <input id="sql-max" type="number" min="1" max="100000" [ngModel]="maxRows()" (ngModelChange)="maxRows.set(+$event)" />
              <label class="check"><input type="checkbox" [ngModel]="explain()" (ngModelChange)="explain.set($event)" /> {{ 'engine.explain' | t }}</label>
              <button type="button" class="primary" [disabled]="busy() || !sql().trim()" (click)="runSql()"><apsi-icon name="sparkle" [live]="busy()" /> {{ 'lab.run' | t }}</button>
              <button type="button" class="ghost" [disabled]="busy() || !sql().trim()" (click)="sqlParquet()"><apsi-icon name="download" /> Parquet</button>
            </div>
          </section>
          @if (error(); as e) {
            <div class="state state-error" role="alert"><apsi-icon name="alert" size="lg" /><p><code>{{ e.code }}</code> {{ e.message }}</p></div>
          }
          @if (sqlResult(); as r) {
            <section class="panel" aria-labelledby="sqlres-h">
              <div class="panel-head">
                <h2 id="sqlres-h"><apsi-icon name="chart" /> {{ 'lab.result' | t }}</h2>
                <span class="muted small">{{ 'engine.timing' | t: { rows: r.table.rows.length, ms: ms(r.elapsed_us) } }}</span>
              </div>
              <div class="table-wrap">
                <table>
                  <caption>
                    {{ r.tables_used.join(', ') || '—' }}
                    @if (r.table.truncated) {
                      · {{ 'engine.truncated' | t }}
                    }
                  </caption>
                  <thead>
                    <tr>
                      @for (c of r.table.columns; track c.name) {
                        <th scope="col">{{ c.name }} @if (c.unit) { <span class="muted">({{ c.unit }})</span> }</th>
                      }
                    </tr>
                  </thead>
                  <tbody>
                    @for (row of r.table.rows; track $index) {
                      <tr>
                        @for (v of row; track $index) {
                          <td [class.num]="isNum(v)">{{ cell(v) }}</td>
                        }
                      </tr>
                    }
                  </tbody>
                </table>
              </div>
              @if (r.logical_plan) {
                <details>
                  <summary>{{ 'engine.plans' | t }}</summary>
                  <pre class="code">{{ r.logical_plan }}</pre>
                  <pre class="code">{{ r.physical_plan }}</pre>
                </details>
              }
            </section>
          }
        </div>
      </div>
    } @else {
      <section class="panel" aria-labelledby="wf-h">
        <h2 id="wf-h"><apsi-icon name="network" /> {{ 'engine.workflow' | t }}</h2>
        <div class="toolbar">
          <label for="wf-example">{{ 'engine.example' | t }}</label>
          <select id="wf-example" [ngModel]="example()" (ngModelChange)="loadExample($event)">
            @for (x of examples; track x.id) {
              <option [value]="x.id">{{ x.label | t }}</option>
            }
            @if (tableExample()) {
              <option value="table">{{ 'engine.exTable' | t }}</option>
            }
          </select>
        </div>
        <label for="wf-json">{{ 'lab.json' | t }}</label>
        <textarea id="wf-json" class="code" rows="14" spellcheck="false" [ngModel]="workflow()" (ngModelChange)="workflow.set($event)" aria-describedby="wf-help"></textarea>
        <p id="wf-help" class="muted small">{{ 'engine.workflowHelp' | t }}</p>
        @if (parseError(); as pe) {
          <p class="state state-error" role="alert"><apsi-icon name="alert" /> {{ 'lab.invalidJson' | t }}: {{ pe }}</p>
        }
        <div class="actions">
          <button type="button" class="ghost" [disabled]="busy() || !!parseError()" (click)="plan()"><apsi-icon name="map" /> {{ 'engine.plan' | t }}</button>
          <button type="button" class="primary" [disabled]="busy() || !!parseError()" (click)="run()"><apsi-icon name="sparkle" [live]="busy()" /> {{ 'lab.run' | t }}</button>
          <button type="button" class="ghost" [disabled]="busy() || !!parseError()" (click)="runParquet()"><apsi-icon name="download" /> Parquet</button>
          <span class="muted small">{{ 'lab.synthetic' | t }}</span>
        </div>
      </section>

      @if (error(); as e) {
        <div class="state state-error" role="alert"><apsi-icon name="alert" size="lg" /><p><code>{{ e.code }}</code> {{ e.message }}</p></div>
      }

      @if (planReport(); as p) {
        <div class="two-col">
          <section class="panel" aria-labelledby="rw-h">
            <h2 id="rw-h"><apsi-icon name="sliders" /> {{ 'engine.rewrites' | t }}</h2>
            @if (p.rewrites.length === 0) {
              <p class="muted">{{ 'engine.noRewrites' | t }}</p>
            } @else {
              <ol class="timeline" role="list">
                @for (r of p.rewrites; track $index) {
                  <li><apsi-icon name="check" /><div><code>{{ r.rule }}</code><p class="small">{{ r.detail }}</p></div></li>
                }
              </ol>
            }
            @for (n of p.notes; track n) {
              <p class="note small"><apsi-icon name="alert" /> {{ n }}</p>
            }
          </section>
          <section class="panel" aria-labelledby="cost-h">
            <h2 id="cost-h"><apsi-icon name="gauge" /> {{ 'engine.costs' | t }}</h2>
            <div class="table-wrap">
              <table>
                <caption class="visually-hidden">{{ 'engine.costs' | t }}</caption>
                <thead>
                  <tr>
                    <th scope="col">{{ 'engine.node' | t }}</th>
                    <th scope="col" class="num">{{ 'engine.estRows' | t }}</th>
                    <th scope="col">{{ 'engine.backend' | t }}</th>
                    <th scope="col">{{ 'engine.reason' | t }}</th>
                  </tr>
                </thead>
                <tbody>
                  @for (c of p.costs; track c.id) {
                    <tr>
                      <th scope="row"><code>{{ c.id }}</code> <span class="muted small">{{ c.op }}</span></th>
                      <td class="num">{{ i18n.number(c.rows) }}</td>
                      <td><span class="badge">{{ backend[c.backend].label | t }}</span></td>
                      <td class="small">{{ c.reason }}</td>
                    </tr>
                  }
                </tbody>
              </table>
            </div>
            <p class="muted small">{{ 'engine.levels' | t: { n: p.levels.length } }}</p>
          </section>
        </div>
      }

      @if (runReport(); as r) {
        <section class="panel" aria-labelledby="prof-h">
          <div class="panel-head">
            <h2 id="prof-h"><apsi-icon name="clock" /> {{ 'engine.profile' | t }}</h2>
            <span class="muted small">{{ 'engine.memory' | t: { ms: ms(r.elapsed_us), peak: kib(r.memory.peak_bytes), limit: kib(r.memory.limit_bytes) } }}</span>
          </div>
          <div class="table-wrap">
            <table>
              <caption class="visually-hidden">{{ 'engine.profile' | t }}</caption>
              <thead>
                <tr>
                  <th scope="col">{{ 'engine.node' | t }}</th>
                  <th scope="col">{{ 'engine.backend' | t }}</th>
                  <th scope="col" class="num">{{ 'engine.rows' | t }}</th>
                  <th scope="col" class="num">ms</th>
                  <th scope="col">{{ 'engine.cache' | t }}</th>
                  <th scope="col">{{ 'engine.note' | t }}</th>
                </tr>
              </thead>
              <tbody>
                @for (n of r.profile; track n.id) {
                  <tr>
                    <th scope="row"><code>{{ n.id }}</code> <span class="muted small">{{ n.op }}</span></th>
                    <td><span class="badge">{{ backend[n.backend].label | t }}</span></td>
                    <td class="num">{{ i18n.number(n.rows) }}</td>
                    <td class="num">{{ ms(n.elapsed_us) }}</td>
                    <td><span class="badge" [attr.data-outcome]="n.cache === 'hit' ? 'allowed' : null">{{ n.cache }}</span></td>
                    <td class="small">{{ n.note ?? '' }}</td>
                  </tr>
                }
              </tbody>
            </table>
          </div>
          <h3><apsi-icon name="shield" /> {{ 'engine.checks' | t }}</h3>
          <ul class="status-list" role="list">
            @for (c of r.checks; track $index) {
              <li [attr.data-outcome]="c.passed ? 'allowed' : 'failed'">
                <apsi-icon [name]="c.passed ? 'check' : 'alert'" />
                <span><code>{{ c.output }}.{{ c.column }}</code> {{ c.rule }}</span>
                <strong>{{ c.passed ? ('engine.passed' | t) : ('engine.violations' | t: { n: c.violations }) }}</strong>
              </li>
            }
          </ul>
        </section>

        @for (o of outputs(); track o.name) {
          <section class="panel" [attr.aria-label]="o.name">
            <div class="panel-head">
              <h2><apsi-icon name="layers" /> <code>{{ o.name }}</code></h2>
              <span class="muted small">{{ 'engine.outputRows' | t: { n: o.table.total_rows } }}</span>
            </div>
            <div class="table-wrap">
              <table>
                <caption class="visually-hidden">{{ o.name }}</caption>
                <thead>
                  <tr>
                    @for (c of o.table.columns; track c.name) {
                      <th scope="col">{{ c.name }} @if (c.unit) { <span class="muted">({{ c.unit }})</span> }</th>
                    }
                  </tr>
                </thead>
                <tbody>
                  @for (row of o.table.rows; track $index) {
                    <tr>
                      @for (v of row; track $index) {
                        <td [class.num]="isNum(v)">{{ cell(v) }}</td>
                      }
                    </tr>
                  }
                </tbody>
              </table>
            </div>
          </section>
        }
      }
    }
  `,
})
export class EnginePage {
  private readonly api = inject(Api);
  private readonly announcer = inject(Announcer);
  protected readonly i18n = inject(I18n);
  protected readonly backend = BACKEND;
  protected readonly examples = ENGINE_EXAMPLES;
  /** `?table=` preloads a query over that table; `?tab=workflow` opens the workflow runner. */
  readonly table = input<string | undefined>(undefined);
  readonly tabParam = input<string | undefined>(undefined, { alias: 'tab' });

  readonly caps = loadable(() => this.api.engine());
  readonly tables = loadable(() => this.api.engineTables(), (v) => v.length === 0);
  readonly tab = signal<Tab>('sql');
  readonly sql = signal('SELECT 1 AS answer');
  readonly maxRows = signal(500);
  readonly explain = signal(false);
  readonly example = signal(ENGINE_EXAMPLES[0].id);
  readonly workflow = signal(JSON.stringify(ENGINE_EXAMPLES[0].workflow, null, 2));
  readonly busy = signal(false);
  readonly error = signal<ApiFailure | null>(null);
  readonly sqlResult = signal<SqlResult | null>(null);
  readonly planReport = signal<PlanReport | null>(null);
  readonly runReport = signal<RunReport | null>(null);

  readonly outputs = computed(() => Object.entries(this.runReport()?.outputs ?? {}).map(([name, table]) => ({ name, table: table as EngineTable })));
  readonly parseError = computed(() => {
    try {
      JSON.parse(this.workflow());
      return '';
    } catch (e) {
      return e instanceof Error ? e.message : String(e);
    }
  });
  /** A summary workflow over the first real table that has a numeric `value` column. */
  readonly tableExample = computed(() => {
    const t = (this.tables.data() ?? []).find((x) => x.columns.some((c) => c.name === 'value' && c.type === 'float64'));
    if (!t) return null;
    return {
      nodes: [
        { id: 'data', op: 'scan', domain: t.domain, entity: t.entity },
        { id: 'measured', op: 'filter', input: 'data', predicate: { not: { is_null: { col: 'value' } } } },
        {
          id: 'summary',
          op: 'aggregate',
          input: 'measured',
          group_by: ['lifecycle'],
          aggregates: ['count', 'mean', 'sd', 'min', 'median', 'max'].map((f) => ({ name: f, func: f, column: 'value' })),
        },
      ],
      outputs: ['summary'],
    };
  });

  constructor() {
    effect(() => {
      const t = this.table();
      const tab = this.tabParam();
      untracked(() => {
        if (t) {
          this.tab.set('sql');
          this.sql.set(`SELECT *\nFROM ${t}\nLIMIT 100`);
        }
        if (tab === 'workflow' || tab === 'sql') this.tab.set(tab);
      });
    });
  }

  useTable(t: EngineTableInfo): void {
    const numeric = t.columns.filter((c) => c.type === 'float64').map((c) => c.name);
    this.sql.set(
      numeric.includes('value')
        ? `SELECT lifecycle, count(*) AS n, avg(value) AS mean, min(value) AS min, max(value) AS max\nFROM ${t.table}\nGROUP BY lifecycle\nORDER BY lifecycle`
        : `SELECT *\nFROM ${t.table}\nLIMIT 100`,
    );
  }

  loadExample(id: string): void {
    this.example.set(id);
    const wf = id === 'table' ? this.tableExample() : ENGINE_EXAMPLES.find((x) => x.id === id)?.workflow;
    if (wf) this.workflow.set(JSON.stringify(wf, null, 2));
    this.planReport.set(null);
    this.runReport.set(null);
    this.error.set(null);
  }

  private start(): void {
    this.busy.set(true);
    this.error.set(null);
  }

  private fail(e: unknown): void {
    const f = ApiFailure.from(e);
    this.busy.set(false);
    this.error.set(f);
    this.announcer.say(`${f.code}: ${f.message}`, 'error');
  }

  runSql(): void {
    this.start();
    this.sqlResult.set(null);
    this.api.engineSql({ sql: this.sql(), max_rows: this.maxRows(), explain: this.explain() }).subscribe({
      next: (r) => {
        this.busy.set(false);
        this.sqlResult.set(r);
        this.announcer.say(this.i18n.t('engine.done', { n: r.table.rows.length }), 'success');
      },
      error: (e: unknown) => this.fail(e),
    });
  }

  private body(): unknown {
    return JSON.parse(this.workflow());
  }

  plan(): void {
    this.start();
    this.runReport.set(null);
    this.api.enginePlan(this.body()).subscribe({
      next: (p) => {
        this.busy.set(false);
        this.planReport.set(p);
      },
      error: (e: unknown) => this.fail(e),
    });
  }

  run(): void {
    this.start();
    this.api.engineRun(this.body()).subscribe({
      next: (r) => {
        this.busy.set(false);
        this.planReport.set(r.plan);
        this.runReport.set(r);
        this.announcer.say(this.i18n.t('engine.ran', { n: r.profile.length, ms: this.ms(r.elapsed_us) }), 'success');
      },
      error: (e: unknown) => this.fail(e),
    });
  }

  runParquet(): void {
    const body = this.body() as { outputs?: string[] };
    this.download(this.api.engineParquet('run', body, body.outputs?.[0]), `${body.outputs?.[0] ?? 'output'}.parquet`);
  }

  sqlParquet(): void {
    this.download(this.api.engineParquet('sql', { sql: this.sql(), max_rows: this.maxRows() }), 'query.parquet');
  }

  private download(obs: ReturnType<Api['engineParquet']>, name: string): void {
    this.start();
    obs.subscribe({
      next: (blob) => {
        this.busy.set(false);
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = name;
        a.click();
        URL.revokeObjectURL(url);
        this.announcer.say(this.i18n.t('records.exported', { format: 'Parquet' }), 'success');
      },
      error: (e: unknown) => this.fail(e),
    });
  }

  ms(us: number): string {
    return this.i18n.number(us / 1000, 2);
  }

  kib(bytes: number): string {
    return `${this.i18n.number(bytes / 1024, 1)} KiB`;
  }

  isNum(v: unknown): boolean {
    return typeof v === 'number';
  }

  cell(v: unknown): string {
    if (v === null || v === undefined) return '—';
    if (typeof v === 'number') return this.i18n.number(v, 6);
    return typeof v === 'string' ? v : JSON.stringify(v);
  }
}
