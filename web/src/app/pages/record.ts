import { JsonPipe } from '@angular/common';
import { Component, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { Auth } from '../core/auth';
import { CatalogStore } from '../core/catalog-store';
import { I18n, TPipe } from '../core/i18n';
import { Announcer, loadable } from '../core/load';
import { ApsiRecord, FieldChange, Lifecycle } from '../core/models';
import { groupOf, iconOf } from '../core/taxonomy';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';

@Component({
  selector: 'apsi-record',
  imports: [JsonPipe, RouterLink, StateView, TPipe, Icon],
  template: `
    <apsi-state [status]="rec.status()" [error]="rec.error()" (retry)="rec.reload()" />
    @if (rec.data(); as r) {
      <nav aria-label="breadcrumb" class="breadcrumb">
        <a routerLink="/domains">{{ 'nav.domains' | t }}</a>
        <span aria-hidden="true">›</span>
        <a [routerLink]="['/domains', r.domain]">{{ store.domainName(r.domain) }}</a>
        <span aria-hidden="true">›</span>
        <a [routerLink]="['/domains', r.domain, r.entity]">{{ store.entityName(r.domain, r.entity) }}</a>
      </nav>
      <header class="page-head" [attr.data-tone]="tone(r.domain)">
        <span class="page-icon"><apsi-icon [name]="icon(r.domain)" [live]="true" size="xl" /></span>
        <div>
          <h1 tabindex="-1">{{ r.name }}</h1>
          <p class="meta-line">
            <span class="badge" [attr.data-lifecycle]="r.lifecycle">{{ r.lifecycle }}</span>
            <span>v{{ r.version }}</span>
            <span>{{ 'record.schema' | t: { n: r.schema_version } }}</span>
            <code>{{ r.id }}</code>
            @if (r.legal_hold) {
              <span class="badge bad"><apsi-icon name="lock" /> {{ 'record.legalHold' | t }}</span>
            }
          </p>
        </div>
        <div class="page-actions">
          @if (labLink(); as q) {
            <a class="button-link ghost" routerLink="/lab" [queryParams]="q"><apsi-icon name="ruler" /> {{ 'record.convert' | t }}</a>
          }
          @if (auth.has('editor')) {
            @if (r.lifecycle !== 'archived') {
              <button type="button" class="ghost" (click)="transition(r, 'archived')"><apsi-icon name="scroll" /> {{ 'record.archive' | t }}</button>
            } @else if (auth.has('admin')) {
              <button type="button" class="ghost" (click)="transition(r, 'active')"><apsi-icon name="history" /> {{ 'record.restore' | t }}</button>
            }
          }
        </div>
      </header>

      <div class="two-col wide-left">
        <div class="side-stack">
          <section class="panel" aria-labelledby="attr-h">
            <h2 id="attr-h"><apsi-icon name="layers" /> {{ 'record.attributes' | t }}</h2>
            @if (attrs(r).length === 0) {
              <p class="muted">{{ 'state.empty' | t }}</p>
            } @else {
              <dl class="attr-grid">
                @for (a of attrs(r); track a[0]) {
                  <div class="attr-card">
                    <dt>{{ a[0] }}</dt>
                    <dd>
                      @if (isScalar(a[1])) {
                        <strong>{{ scalar(a[1]) }}</strong>
                        @if (a[0] === 'value' && r.attributes['unit']) {
                          <span class="muted"> {{ r.attributes['unit'] }}</span>
                        }
                      } @else {
                        <code class="json">{{ a[1] | json }}</code>
                      }
                    </dd>
                  </div>
                }
              </dl>
            }
            <p class="meta-line small muted">
              <span><apsi-icon name="users" /> {{ 'record.owner' | t }}: {{ r.owner }}</span>
              <span><apsi-icon name="clock" /> {{ 'records.updated' | t }}: <time [attr.datetime]="r.updated_at">{{ i18n.date(r.updated_at) }}</time></span>
              @if (r.source) {
                <span><apsi-icon name="plug" /> {{ r.source.system }}:{{ r.source.id }}</span>
              }
            </p>
            @if (r.tags.length) {
              <p>
                @for (t of r.tags; track t) {
                  <span class="tag">{{ t }}</span>
                }
              </p>
            }
          </section>

          <section class="panel" aria-labelledby="hist-h">
            <h2 id="hist-h"><apsi-icon name="history" /> {{ 'record.history' | t }}</h2>
            <apsi-state [status]="hist.status()" [error]="hist.error()" (retry)="hist.reload()" />
            <ol class="timeline" reversed>
              @for (v of hist.data() ?? []; track v.version) {
                <li>
                  <span class="timeline-dot">v{{ v.version }}</span>
                  <div>
                    <p>{{ v.change }} · {{ v.actor }}</p>
                    <p class="muted small"><time [attr.datetime]="v.at">{{ i18n.date(v.at) }}</time> · {{ v.lifecycle }}</p>
                    <div class="actions">
                      @if (v.version > 1) {
                        <button type="button" class="link" (click)="showDiff(r.id, v.version)" [attr.aria-expanded]="diffFor() === v.version">{{ 'record.diff' | t }}</button>
                      }
                      @if (v.version < r.version && r.lifecycle !== 'archived' && auth.has('editor')) {
                        <button type="button" class="link" (click)="rollback(r, v.version)">{{ 'record.rollback' | t }}</button>
                      }
                    </div>
                    @if (diffFor() === v.version) {
                      <div class="table-wrap">
                        <table class="diff">
                          <caption class="visually-hidden">{{ 'record.diff' | t }}</caption>
                          <thead>
                            <tr><th scope="col">{{ 'record.field' | t }}</th><th scope="col">{{ 'record.before' | t }}</th><th scope="col">{{ 'record.after' | t }}</th></tr>
                          </thead>
                          <tbody>
                            @for (c of diff(); track c.path) {
                              <tr><th scope="row">{{ c.path }}</th><td><del>{{ c.before | json }}</del></td><td><ins>{{ c.after | json }}</ins></td></tr>
                            }
                          </tbody>
                        </table>
                      </div>
                    }
                  </div>
                </li>
              }
            </ol>
          </section>
        </div>

        <aside class="panel" aria-labelledby="prov-h">
          <h2 id="prov-h"><apsi-icon name="tree" /> {{ 'record.provenance' | t }}</h2>
          <ol class="timeline">
            @for (p of r.provenance; track $index) {
              <li>
                <apsi-icon [name]="p.actor_kind === 'service' ? 'chip' : 'users'" />
                <div>
                  <p><code>{{ p.action }}</code> · {{ p.actor }} <span class="muted">({{ p.actor_kind }})</span></p>
                  <p class="muted small"><time [attr.datetime]="p.at">{{ i18n.date(p.at) }}</time></p>
                  @if (p.source) {
                    <p class="small"><apsi-icon name="plug" /> {{ p.source.system }}:{{ p.source.id }}</p>
                  }
                  @if (p.transformation) {
                    <p class="small"><apsi-icon name="sliders" /> {{ p.transformation }}</p>
                  }
                </div>
              </li>
            }
          </ol>
        </aside>
      </div>
    }
  `,
})
export class RecordPage {
  private readonly api = inject(Api);
  private readonly announcer = inject(Announcer);
  protected readonly auth = inject(Auth);
  protected readonly i18n = inject(I18n);
  protected readonly store = inject(CatalogStore);
  readonly id = input.required<string>();
  readonly rec = loadable(() => this.api.record(this.id()), undefined, false);
  readonly hist = loadable(() => this.api.history(this.id()), undefined, false);
  readonly diffFor = signal<number | null>(null);
  readonly diff = signal<FieldChange[]>([]);
  private readonly unitPair = computed(() => {
    const a = this.rec.data()?.attributes;
    const v = a?.['value'];
    const u = a?.['unit'];
    return typeof v === 'number' && typeof u === 'string' && u ? { value: v, unit: u } : null;
  });

  constructor() {
    this.store.ensure();
    effect(() => {
      this.id();
      untracked(() => this.refresh());
    });
  }

  private refresh(): void {
    this.diffFor.set(null);
    this.rec.reload();
    this.hist.reload();
  }

  icon(domain: string) {
    return iconOf(domain);
  }

  tone(domain: string) {
    return groupOf(domain).tone;
  }

  attrs(r: ApsiRecord): [string, unknown][] {
    return Object.entries(r.attributes);
  }

  isScalar(v: unknown): boolean {
    return v === null || ['string', 'number', 'boolean'].includes(typeof v);
  }

  scalar(v: unknown): string {
    if (typeof v === 'number') return this.i18n.number(v, 6);
    if (typeof v === 'string' || typeof v === 'boolean') return String(v);
    return '—';
  }

  /** Hand-off to the Science Lab's unit converter, prefilled with this record's value. */
  readonly labLink = computed(() => {
    const p = this.unitPair();
    if (!p) return null;
    return { fn: 'units.convert', input: JSON.stringify({ value: p.value, from: p.unit, to: p.unit }) };
  });

  private handle(e: unknown): void {
    const f = ApiFailure.from(e);
    this.announcer.say(f.code === 'version_conflict' ? this.i18n.t('record.conflict') : `${f.code}: ${f.message}`, 'error');
    this.refresh();
  }

  transition(r: ApsiRecord, to: Lifecycle): void {
    this.api.transition(r.id, to, r.version).subscribe({
      next: () => {
        this.announcer.say(this.i18n.t(to === 'archived' ? 'record.archived' : 'record.restored'), 'success');
        this.refresh();
      },
      error: (e: unknown) => this.handle(e),
    });
  }

  rollback(r: ApsiRecord, version: number): void {
    this.api.rollback(r.id, version, r.version).subscribe({
      next: () => {
        this.announcer.say(this.i18n.t('record.rolledBack', { n: version }), 'success');
        this.refresh();
      },
      error: (e: unknown) => this.handle(e),
    });
  }

  showDiff(id: string, version: number): void {
    if (this.diffFor() === version) {
      this.diffFor.set(null);
      return;
    }
    this.api.diff(id, version - 1, version).subscribe({
      next: (d) => {
        this.diff.set(d);
        this.diffFor.set(version);
      },
      error: (e: unknown) => this.handle(e),
    });
  }
}
