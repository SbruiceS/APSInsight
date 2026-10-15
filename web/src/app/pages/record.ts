import { JsonPipe } from '@angular/common';
import { Component, effect, inject, input, signal, untracked } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api, ApiFailure } from '../core/api.service';
import { Auth } from '../core/auth';
import { I18n, TPipe } from '../core/i18n';
import { Announcer, loadable } from '../core/load';
import { ApsiRecord, FieldChange, Lifecycle } from '../core/models';
import { StateView } from '../shared/state-view';

@Component({
  selector: 'apsi-record',
  imports: [JsonPipe, RouterLink, StateView, TPipe],
  template: `
    <apsi-state [status]="rec.status()" [error]="rec.error()" (retry)="rec.reload()" />
    @if (rec.data(); as r) {
      <nav aria-label="breadcrumb" class="muted">
        <a [routerLink]="['/domains', r.domain]">{{ r.domain }}</a> /
        <a [routerLink]="['/domains', r.domain, r.entity]">{{ r.entity }}</a>
      </nav>
      <h1 tabindex="-1">{{ r.name }}</h1>
      <p>
        <span class="badge">{{ r.lifecycle }}</span>
        v{{ r.version }} · schema v{{ r.schema_version }} · <code>{{ r.id }}</code>
        @if (r.legal_hold) {
          · <span class="badge bad">{{ 'record.legalHold' | t }}</span>
        }
      </p>
      @if (auth.has('editor')) {
        <div class="actions">
          @if (r.lifecycle !== 'archived') {
            <button type="button" (click)="transition(r, 'archived')">{{ 'record.archive' | t }}</button>
          } @else if (auth.has('admin')) {
            <button type="button" (click)="transition(r, 'active')">{{ 'record.restore' | t }}</button>
          }
        </div>
      }

      <h2>{{ 'record.attributes' | t }}</h2>
      <dl class="kv">
        @for (a of attrs(r); track a[0]) {
          <dt>{{ a[0] }}</dt>
          <dd><code>{{ a[1] | json }}</code></dd>
        }
      </dl>

      <h2>{{ 'record.provenance' | t }}</h2>
      <ol class="activity">
        @for (p of r.provenance; track $index) {
          <li>
            <time [attr.datetime]="p.at">{{ i18n.date(p.at) }}</time> · {{ p.actor }} ({{ p.actor_kind }}) · <code>{{ p.action }}</code>
            @if (p.source) { · {{ p.source.system }}:{{ p.source.id }} }
            @if (p.transformation) { · {{ p.transformation }} }
          </li>
        }
      </ol>

      <h2>{{ 'record.history' | t }}</h2>
      <apsi-state [status]="hist.status()" [error]="hist.error()" (retry)="hist.reload()" />
      <ol class="activity" reversed>
        @for (v of hist.data() ?? []; track v.version) {
          <li>
            v{{ v.version }} · <time [attr.datetime]="v.at">{{ i18n.date(v.at) }}</time> · {{ v.actor }} · {{ v.change }}
            @if (v.version > 1) {
              <button type="button" class="link" (click)="showDiff(r.id, v.version)" [attr.aria-expanded]="diffFor() === v.version">{{ 'record.diff' | t }}</button>
            }
            @if (v.version < r.version && r.lifecycle !== 'archived' && auth.has('editor')) {
              <button type="button" class="link" (click)="rollback(r, v.version)">{{ 'record.rollback' | t }}</button>
            }
            @if (diffFor() === v.version) {
              <table>
                <caption class="visually-hidden">{{ 'record.diff' | t }}</caption>
                <tbody>
                  @for (c of diff(); track c.path) {
                    <tr><th scope="row">{{ c.path }}</th><td><del>{{ c.before | json }}</del></td><td><ins>{{ c.after | json }}</ins></td></tr>
                  }
                </tbody>
              </table>
            }
          </li>
        }
      </ol>
    }
  `,
})
export class RecordPage {
  private readonly api = inject(Api);
  private readonly announcer = inject(Announcer);
  protected readonly auth = inject(Auth);
  protected readonly i18n = inject(I18n);
  readonly id = input.required<string>();
  readonly rec = loadable(() => this.api.record(this.id()), undefined, false);
  readonly hist = loadable(() => this.api.history(this.id()), undefined, false);
  readonly diffFor = signal<number | null>(null);
  readonly diff = signal<FieldChange[]>([]);

  constructor() {
    effect(() => {
      this.id();
      untracked(() => this.refresh());
    });
  }

  private refresh(): void {
    this.rec.reload();
    this.hist.reload();
  }

  attrs(r: ApsiRecord): [string, unknown][] {
    return Object.entries(r.attributes);
  }

  private handle(e: unknown): void {
    const f = ApiFailure.from(e);
    this.announcer.say(f.code === 'version_conflict' ? this.i18n.t('record.conflict') : `${f.code}: ${f.message}`);
    this.refresh();
  }

  transition(r: ApsiRecord, to: Lifecycle): void {
    this.api.transition(r.id, to, r.version).subscribe({
      next: () => {
        this.announcer.say(this.i18n.t(to === 'archived' ? 'record.archived' : 'record.restored'));
        this.refresh();
      },
      error: (e: unknown) => this.handle(e),
    });
  }

  rollback(r: ApsiRecord, version: number): void {
    this.api.rollback(r.id, version, r.version).subscribe({ next: () => this.refresh(), error: (e: unknown) => this.handle(e) });
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
