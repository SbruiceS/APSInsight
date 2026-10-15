import { Injectable, computed, inject, signal } from '@angular/core';
import { Api } from './api.service';
import { Catalog, Domain } from './models';

/**
 * The feature register's catalogue (100 domains, 800 entity kinds) is static
 * for a release, so it is fetched once on demand and shared by the shell, the
 * command palette and every page. Reading `catalog()` never triggers a request;
 * call `ensure()` to load it.
 */
@Injectable({ providedIn: 'root' })
export class CatalogStore {
  private readonly api = inject(Api);
  private requested = false;
  readonly catalog = signal<Catalog | null>(null);
  readonly failed = signal(false);
  private readonly byDomain = computed(() => new Map((this.catalog()?.domains ?? []).map((d) => [d.slug, d])));

  ensure(): void {
    if (this.requested && !this.failed()) return;
    this.requested = true;
    this.failed.set(false);
    this.api.catalog().subscribe({ next: (c) => this.catalog.set(c), error: () => this.failed.set(true) });
  }

  domain(slug: string): Domain | undefined {
    return this.byDomain().get(slug);
  }

  domainName(slug: string): string {
    return this.domain(slug)?.name ?? slug;
  }

  entityName(domain: string, entity: string): string {
    return this.domain(domain)?.entities.find((e) => e.slug === entity)?.name ?? entity;
  }
}
