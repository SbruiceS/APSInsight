import { JsonPipe } from '@angular/common';
import { Component, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterLink } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { I18n, TPipe } from '../core/i18n';
import { LAB_EXAMPLES, LAB_FAMILIES, familyOf } from '../core/lab-examples';
import { Announcer, loadable } from '../core/load';
import { ComputeFunction } from '../core/models';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';

interface Run {
  id: number;
  fn: string;
  input: string;
  ok: boolean;
  summary: string;
  at: Date;
}

/** Science Lab: run any registered compute function with validated JSON input. */
@Component({
  selector: 'apsi-lab',
  imports: [FormsModule, JsonPipe, RouterLink, StateView, TPipe, Icon],
  template: `
    <header class="page-head" data-tone="sky">
      <span class="page-icon"><apsi-icon name="flask" [live]="true" size="xl" /></span>
      <div>
        <p class="eyebrow">{{ 'nav.science' | t }}</p>
        <h1 tabindex="-1">{{ 'lab.title' | t }}</h1>
        <p class="muted">{{ 'lab.lead' | t }}</p>
      </div>
    </header>

    <apsi-state [status]="fns.status()" [error]="fns.error()" (retry)="fns.reload()" />
    @if (fns.data()) {
      <div class="lab">
        <aside class="panel lab-list" aria-labelledby="fn-h">
          <h2 id="fn-h">{{ 'lab.functions' | t }}</h2>
          <label for="fn-filter" class="visually-hidden">{{ 'lab.filter' | t }}</label>
          <span class="input-icon">
            <apsi-icon name="search" />
            <input id="fn-filter" type="search" [placeholder]="'lab.filter' | t" [value]="filter()" (input)="filter.set($any($event.target).value)" />
          </span>
          @for (g of grouped(); track g.prefix) {
            <h3 class="lab-family"><apsi-icon [name]="g.icon" /> {{ g.label | t }}</h3>
            <ul role="list" class="fn-list">
              @for (f of g.items; track f.name) {
                <li>
                  <button type="button" class="fn-button" [attr.aria-pressed]="f.name === selected()" (click)="choose(f.name)">
                    <code>{{ f.name }}</code>
                    <span class="muted small">{{ f.description }}</span>
                  </button>
                </li>
              }
            </ul>
          }
          @if (grouped().length === 0) {
            <p class="muted">{{ 'lab.noMatch' | t }}</p>
          }
        </aside>

        <div class="side-stack">
          @if (current(); as f) {
            <section class="panel" aria-labelledby="run-h">
              <div class="panel-head">
                <h2 id="run-h"><apsi-icon [name]="familyIcon(f.name)" [live]="busy()" /> <code>{{ f.name }}</code></h2>
              </div>
              <p>{{ f.description }}</p>
              <dl class="sig">
                <div><dt>{{ 'lab.input' | t }}</dt><dd><code>{{ f.input }}</code></dd></div>
                <div><dt>{{ 'lab.output' | t }}</dt><dd><code>{{ f.output }}</code></dd></div>
              </dl>
              <label for="lab-input">{{ 'lab.json' | t }}</label>
              <textarea id="lab-input" class="code" rows="10" spellcheck="false" [ngModel]="text()" (ngModelChange)="text.set($event)" aria-describedby="lab-input-help"></textarea>
              <p id="lab-input-help" class="muted small">{{ 'lab.jsonHelp' | t }}</p>
              @if (parseError(); as pe) {
                <p class="state state-error" role="alert"><apsi-icon name="alert" /> {{ 'lab.invalidJson' | t }}: {{ pe }}</p>
              }
              <div class="actions">
                <button type="button" class="primary" [disabled]="busy() || !!parseError()" (click)="run()"><apsi-icon name="sparkle" [live]="busy()" /> {{ 'lab.run' | t }}</button>
                <button type="button" class="ghost" [disabled]="!hasExample()" (click)="loadExample()"><apsi-icon name="seed" /> {{ 'lab.example' | t }}</button>
                <button type="button" class="ghost" [disabled]="!!parseError()" (click)="format()"><apsi-icon name="sliders" /> {{ 'lab.format' | t }}</button>
              </div>
              @if (hasExample()) {
                <p class="muted small">{{ 'lab.synthetic' | t }}</p>
              }
            </section>

            <section class="panel" aria-labelledby="res-h" aria-live="polite">
              <h2 id="res-h"><apsi-icon name="chart" /> {{ 'lab.result' | t }}</h2>
              @if (error(); as e) {
                <div class="state state-error" role="alert">
                  <apsi-icon name="alert" size="lg" />
                  <div>
                    <p><code>{{ e.code }}</code> {{ e.message }}</p>
                  </div>
                </div>
              } @else if (hasResult()) {
                @if (scalarResult(); as s) {
                  <p class="result-big">{{ s }}</p>
                } @else if (entries(); as kv) {
                  <dl class="attr-grid">
                    @for (e of kv; track e[0]) {
                      <div class="attr-card"><dt>{{ e[0] }}</dt><dd><strong>{{ e[1] }}</strong></dd></div>
                    }
                  </dl>
                }
                <details [open]="!scalarResult() && !entries()">
                  <summary>{{ 'lab.raw' | t }}</summary>
                  <pre class="code">{{ result() | json }}</pre>
                </details>
              } @else {
                <p class="muted">{{ 'lab.noResult' | t }}</p>
              }
            </section>
          } @else {
            <section class="panel empty-hint">
              <apsi-icon name="flask" [live]="true" size="xl" />
              <p>{{ 'lab.pick' | t }}</p>
            </section>
          }

          <section class="panel" aria-labelledby="hist-h">
            <h2 id="hist-h"><apsi-icon name="history" /> {{ 'lab.history' | t }}</h2>
            @if (runs().length === 0) {
              <p class="muted">{{ 'lab.noHistory' | t }}</p>
            } @else {
              <ol class="timeline" role="list">
                @for (r of runs(); track r.id) {
                  <li [attr.data-outcome]="r.ok ? 'allowed' : 'failed'">
                    <apsi-icon [name]="r.ok ? 'check' : 'alert'" />
                    <div>
                      <button type="button" class="link" (click)="restore(r)"><code>{{ r.fn }}</code></button>
                      <p class="muted small"><time>{{ r.at.toLocaleTimeString() }}</time> · {{ r.summary }}</p>
                    </div>
                  </li>
                }
              </ol>
            }
            <p class="small"><a routerLink="/features" [queryParams]="{ q: 'compute' }">{{ 'lab.evidence' | t }} <apsi-icon name="arrowRight" /></a></p>
          </section>
        </div>
      </div>
    }
  `,
})
export class LabPage {
  private readonly api = inject(Api);
  private readonly router = inject(Router);
  private readonly announcer = inject(Announcer);
  protected readonly i18n = inject(I18n);
  /** Bound from `?fn=` and `?input=` so other pages can hand work to the lab. */
  readonly fn = input<string | undefined>(undefined);
  readonly input = input<string | undefined>(undefined);

  readonly fns = loadable(() => this.api.computeFunctions());
  readonly filter = signal('');
  readonly selected = signal('');
  readonly text = signal('');
  readonly busy = signal(false);
  readonly result = signal<unknown>(undefined);
  readonly hasResult = signal(false);
  readonly error = signal<ApiFailure | null>(null);
  readonly runs = signal<Run[]>([]);
  private seq = 0;
  /** Query params this page wrote itself; echoing them back must not reset the editor. */
  private pushed = { fn: '', input: '' };

  readonly current = computed<ComputeFunction | undefined>(() => (this.fns.data() ?? []).find((f) => f.name === this.selected()));
  readonly hasExample = computed(() => this.selected() in LAB_EXAMPLES);
  readonly grouped = computed(() => {
    const q = this.filter().toLowerCase().trim();
    const all = (this.fns.data() ?? []).filter((f) => !q || (f.name + ' ' + f.description).toLowerCase().includes(q));
    const order = [...LAB_FAMILIES.map((f) => f.prefix)];
    for (const f of all) {
      const p = familyOf(f.name).prefix;
      if (!order.includes(p)) order.push(p);
    }
    return order
      .map((p) => ({ ...familyOf(p + '.x'), items: all.filter((f) => familyOf(f.name).prefix === p) }))
      .filter((g) => g.items.length > 0);
  });
  readonly parseError = computed(() => {
    const t = this.text().trim();
    if (!t) return this.i18n.t('lab.empty');
    try {
      JSON.parse(t);
      return '';
    } catch (e) {
      return e instanceof Error ? e.message : String(e);
    }
  });
  readonly scalarResult = computed(() => {
    const r = this.result();
    if (r === null) return '—';
    if (typeof r === 'number') return this.i18n.number(r, 6);
    if (typeof r === 'string' || typeof r === 'boolean') return String(r);
    return '';
  });
  readonly entries = computed<[string, string][] | null>(() => {
    const r = this.result();
    if (!r || typeof r !== 'object' || Array.isArray(r)) return null;
    // Scalar fields become cards; arrays and nested objects stay in the raw JSON.
    const rows = Object.entries(r as Record<string, unknown>).filter(([, v]) => v === null || ['number', 'string', 'boolean'].includes(typeof v));
    if (rows.length === 0) return null;
    return rows.map(([k, v]) => [k, typeof v === 'number' ? this.i18n.number(v, 6) : v === null ? '—' : String(v as string | boolean)]);
  });

  constructor() {
    effect(() => {
      const fn = this.fn();
      const given = this.input();
      untracked(() => {
        if (!fn || (fn === this.pushed.fn && (given ?? '') === this.pushed.input)) return;
        this.selected.set(fn);
        this.text.set(given ? this.pretty(given) : this.example(fn));
        this.clear();
      });
    });
  }

  familyIcon(fn: string) {
    return familyOf(fn).icon;
  }

  choose(name: string): void {
    if (name === this.selected()) return;
    this.selected.set(name);
    this.text.set(this.example(name));
    this.clear();
    this.sync(name, null);
  }

  loadExample(): void {
    this.text.set(this.example(this.selected()));
  }

  format(): void {
    this.text.set(this.pretty(this.text()));
  }

  run(): void {
    const fn = this.selected();
    let body: unknown;
    try {
      body = JSON.parse(this.text());
    } catch {
      return;
    }
    this.busy.set(true);
    this.clear();
    const compact = JSON.stringify(body);
    this.sync(fn, compact.length <= 1500 ? compact : null);
    this.api.compute(fn, body).subscribe({
      next: (r) => {
        this.busy.set(false);
        this.result.set(r);
        this.hasResult.set(true);
        this.record(fn, compact, true, this.summarise(r));
        this.announcer.say(this.i18n.t('lab.done', { fn }), 'success');
      },
      error: (e: unknown) => {
        const f = ApiFailure.from(e);
        this.busy.set(false);
        this.error.set(f);
        this.record(fn, compact, false, f.code);
        this.announcer.say(`${f.code}: ${f.message}`, 'error');
      },
    });
  }

  restore(r: Run): void {
    this.selected.set(r.fn);
    this.text.set(this.pretty(r.input));
    this.clear();
  }

  private sync(fn: string, input: string | null): void {
    this.pushed = { fn, input: input ?? '' };
    void this.router.navigate([], { queryParams: { fn, input }, replaceUrl: true });
  }

  private record(fn: string, input: string, ok: boolean, summary: string): void {
    this.runs.update((rs) => [{ id: ++this.seq, fn, input, ok, summary, at: new Date() }, ...rs].slice(0, 12));
  }

  private summarise(r: unknown): string {
    if (typeof r === 'number') return this.i18n.number(r, 4);
    if (r === null || typeof r === 'string' || typeof r === 'boolean') return String(r);
    const s = JSON.stringify(r);
    return s.length > 60 ? s.slice(0, 57) + '…' : s;
  }

  private clear(): void {
    this.error.set(null);
    this.result.set(undefined);
    this.hasResult.set(false);
  }

  private example(fn: string): string {
    return fn in LAB_EXAMPLES ? JSON.stringify(LAB_EXAMPLES[fn], null, 2) : '{}';
  }

  private pretty(text: string): string {
    try {
      return JSON.stringify(JSON.parse(text), null, 2);
    } catch {
      return text;
    }
  }
}
