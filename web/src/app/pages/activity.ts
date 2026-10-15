import { Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { I18n, TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { AuditEvent } from '../core/models';
import { Icon } from '../shared/icon';
import { StateView } from '../shared/state-view';

@Component({
  selector: 'apsi-notifications',
  imports: [RouterLink, StateView, TPipe, Icon],
  template: `
    <header class="page-head" data-tone="clay">
      <span class="page-icon"><apsi-icon name="bell" [live]="true" size="xl" /></span>
      <div>
        <p class="eyebrow">{{ 'nav.activity' | t }}</p>
        <h1 tabindex="-1">{{ 'notifications.title' | t }}</h1>
        <p class="muted">{{ 'notifications.lead' | t }}</p>
      </div>
    </header>
    <section class="panel">
      <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
      @if (state.status() === 'ready') {
        <ol class="timeline" role="list">
          @for (n of state.data() ?? []; track n.id) {
            <li>
              <apsi-icon name="bell" />
              <div>
                <a [routerLink]="['/records', n.record_id]">{{ n.message }}</a>
                <p class="muted small"><time [attr.datetime]="n.at">{{ i18n.date(n.at) }}</time> · <code>{{ n.event }}</code> · {{ n.rule_id }}</p>
              </div>
            </li>
          }
        </ol>
      }
    </section>
  `,
})
export class NotificationsPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  readonly state = loadable(() => this.api.notifications(), (v) => v.length === 0);
}

@Component({
  selector: 'apsi-audit',
  imports: [RouterLink, StateView, TPipe, Icon],
  template: `
    <header class="page-head" data-tone="slate">
      <span class="page-icon"><apsi-icon name="scroll" [live]="true" size="xl" /></span>
      <div>
        <p class="eyebrow">{{ 'nav.activity' | t }}</p>
        <h1 tabindex="-1">{{ 'audit.title' | t }}</h1>
        <p class="muted">{{ 'audit.lead' | t }}</p>
      </div>
      <div class="page-actions">
        @if (verify.data(); as v) {
          @if (v.valid) {
            <span class="badge good"><apsi-icon name="shield" [live]="true" /> {{ 'audit.verified' | t: { n: v.events ?? 0 } }}</span>
          } @else {
            <span class="badge bad"><apsi-icon name="alert" /> {{ 'audit.broken' | t: { n: v.broken_at ?? 0 } }}</span>
          }
        } @else if (verify.status() === 'loading') {
          <span class="badge"><apsi-icon name="shield" [live]="true" /> {{ 'audit.verifying' | t }}</span>
        }
        <button type="button" class="ghost" (click)="verify.reload()"><apsi-icon name="check" /> {{ 'audit.verify' | t }}</button>
      </div>
    </header>

    <div class="toolbar" role="search">
      <label for="audit-q">{{ 'audit.filter' | t }}</label>
      <span class="input-icon"><apsi-icon name="search" /><input id="audit-q" type="search" [value]="q()" (input)="q.set($any($event.target).value)" /></span>
      <label for="audit-outcome">{{ 'audit.outcome' | t }}</label>
      <select id="audit-outcome" [value]="outcome()" (change)="outcome.set($any($event.target).value)">
        <option value="">{{ 'features.all' | t }}</option>
        <option value="allowed">allowed</option>
        <option value="denied">denied</option>
        <option value="failed">failed</option>
      </select>
    </div>

    <section class="panel">
      <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
      @if (state.status() === 'ready') {
        <div class="table-wrap">
          <table>
            <caption>{{ 'audit.shown' | t: { n: rows().length, total: (state.data() ?? []).length } }}</caption>
            <thead>
              <tr>
                <th scope="col">#</th>
                <th scope="col">{{ 'audit.at' | t }}</th>
                <th scope="col">{{ 'audit.actor' | t }}</th>
                <th scope="col">{{ 'audit.action' | t }}</th>
                <th scope="col">{{ 'audit.target' | t }}</th>
                <th scope="col">{{ 'audit.outcome' | t }}</th>
              </tr>
            </thead>
            <tbody>
              @for (e of rows(); track e.seq) {
                <tr [class.denied]="e.outcome !== 'allowed'">
                  <td>{{ e.seq }}</td>
                  <td><time [attr.datetime]="e.at">{{ i18n.date(e.at) }}</time></td>
                  <td>{{ e.actor }}</td>
                  <td><code>{{ e.action }}</code></td>
                  <td>
                    @if (recordId(e); as id) {
                      <a [routerLink]="['/records', id]"><code>{{ e.target }}</code></a>
                    } @else {
                      <code>{{ e.target }}</code>
                    }
                  </td>
                  <td>
                    <span class="badge" [attr.data-outcome]="e.outcome"><apsi-icon [name]="e.outcome === 'allowed' ? 'check' : 'alert'" /> {{ e.outcome }}</span>
                    @if (e.detail) {
                      <span class="muted small"> {{ e.detail }}</span>
                    }
                  </td>
                </tr>
              }
            </tbody>
          </table>
        </div>
      }
    </section>
  `,
})
export class AuditPage {
  private readonly api = inject(Api);
  protected readonly i18n = inject(I18n);
  readonly state = loadable(() => this.api.audit(), (v) => v.length === 0);
  readonly verify = loadable(() => this.api.auditVerify());
  readonly q = signal('');
  readonly outcome = signal('');
  readonly rows = computed(() => {
    const q = this.q().toLowerCase().trim();
    const o = this.outcome();
    return (this.state.data() ?? []).filter(
      (e) => (!o || e.outcome === o) && (!q || `${e.actor} ${e.action} ${e.target} ${e.detail ?? ''}`.toLowerCase().includes(q)),
    );
  });

  /** Audit targets that name a record id (`rec_…`) link to that record. */
  recordId(e: AuditEvent): string | null {
    const m = /\b(rec_[0-9a-f]{32})\b/.exec(e.target);
    return m ? m[1] : null;
  }
}
