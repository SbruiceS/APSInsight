import { Component, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api } from '../core/api.service';
import { TPipe } from '../core/i18n';
import { loadable } from '../core/load';
import { IntegrationReport } from '../core/models';
import { Icon } from '../shared/icon';
import { IconName } from '../shared/icons';
import { StateView } from '../shared/state-view';

const PRODUCT_ICON: Record<string, IconName> = {
  agriql: 'database',
  'kutajkutambi-ai': 'sparkle',
  agricuda: 'chip',
  apsnetwork: 'network',
  'nityaga-rtos': 'satellite',
  accelerator: 'gauge',
};

/** Register domain holding each product's contract features (mirrors `Product::register_domain`). */
const PRODUCT_DOMAIN: Record<string, string> = {
  agriql: 'future-agriql-integration-contract',
  'kutajkutambi-ai': 'future-kutajkutambi-ai-integration-contract',
  agricuda: 'future-agricuda-integration-contract',
  apsnetwork: 'future-apsnetwork-integration-contract',
  'nityaga-rtos': 'future-nityaga-rtos-integration-contract',
  accelerator: 'hardware-and-fpga-research-interfaces',
};

/** ASPF integration contracts and their live availability. All are optional. */
@Component({
  selector: 'apsi-integrations',
  imports: [RouterLink, StateView, TPipe, Icon],
  template: `
    <header class="page-head" data-tone="berry">
      <span class="page-icon"><apsi-icon name="network" [live]="true" size="xl" /></span>
      <div>
        <p class="eyebrow">{{ 'nav.governance' | t }}</p>
        <h1 tabindex="-1">{{ 'integrations.title' | t }}</h1>
        <p class="muted">{{ 'integrations.lead' | t }}</p>
      </div>
    </header>

    <aside class="note" role="note">
      <apsi-icon name="plug" />
      <p>{{ 'integrations.optional' | t }}</p>
    </aside>

    <apsi-state [status]="state.status()" [error]="state.error()" (retry)="state.reload()" />
    @if (state.data(); as list) {
      <ul class="integration-grid" role="list">
        @for (p of list; track p.product) {
          <li class="integration-card" [attr.data-state]="p.availability.state">
            <div class="entity-head">
              <span class="domain-icon"><apsi-icon [name]="icon(p.product)" size="lg" /></span>
              <h2 class="h3">{{ p.product_name }}</h2>
            </div>
            <p class="avail">
              @if (p.availability.state === 'available') {
                <span class="badge good"><apsi-icon name="check" /> {{ 'integrations.available' | t }}</span>
              } @else {
                <span class="badge warn"><apsi-icon name="alert" /> {{ 'integrations.unavailable' | t }}</span>
              }
              <span class="muted small">{{ 'integrations.contract' | t: { v: p.contract_version } }}</span>
            </p>
            @if (reason(p); as r) {
              <p class="small">{{ r }}</p>
            }
            <h3 class="small muted">{{ 'integrations.operations' | t }}</h3>
            <p>
              @for (o of p.operations; track o) {
                <code class="tag">{{ o }}</code>
              }
            </p>
            <div class="entity-actions">
              <a [routerLink]="['/features']" [queryParams]="{ domain: domain(p.product) }">{{ 'integrations.features' | t }} <apsi-icon name="arrowRight" /></a>
            </div>
          </li>
        }
      </ul>
    }
  `,
})
export class IntegrationsPage {
  private readonly api = inject(Api);
  readonly state = loadable(() => this.api.integrations(), (v) => v.length === 0);

  icon(product: string): IconName {
    return PRODUCT_ICON[product] ?? 'plug';
  }

  domain(product: string): string {
    return PRODUCT_DOMAIN[product] ?? '';
  }

  reason(p: IntegrationReport): string {
    return p.availability.state === 'unavailable' ? p.availability.reason : '';
  }
}
