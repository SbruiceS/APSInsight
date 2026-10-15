import { Component, ElementRef, Injectable, computed, effect, inject, signal, viewChild } from '@angular/core';
import { Router } from '@angular/router';
import { CatalogStore } from '../core/catalog-store';
import { I18n, I18nKey, TPipe } from '../core/i18n';
import { iconOf } from '../core/taxonomy';
import { Icon } from './icon';
import { IconName } from './icons';

export interface NavPage {
  path: string;
  label: I18nKey;
  icon: IconName;
}

/** Every top-level destination; used by the sidebar and the command palette. */
export const NAV_SECTIONS: { label: I18nKey; pages: NavPage[] }[] = [
  {
    label: 'nav.workspace',
    pages: [
      { path: '/', label: 'nav.overview', icon: 'home' },
      { path: '/domains', label: 'nav.domains', icon: 'field' },
    ],
  },
  {
    label: 'nav.science',
    pages: [{ path: '/lab', label: 'nav.lab', icon: 'flask' }],
  },
  {
    label: 'nav.governance',
    pages: [
      { path: '/status', label: 'nav.status', icon: 'gauge' },
      { path: '/features', label: 'nav.features', icon: 'book' },
      { path: '/integrations', label: 'nav.integrations', icon: 'network' },
    ],
  },
  {
    label: 'nav.activity',
    pages: [
      { path: '/notifications', label: 'nav.notifications', icon: 'bell' },
      { path: '/audit', label: 'nav.audit', icon: 'scroll' },
    ],
  },
];

interface PaletteItem {
  id: string;
  label: string;
  hint: string;
  icon: IconName;
  commands: string[];
  haystack: string;
}

@Injectable({ providedIn: 'root' })
export class PaletteState {
  readonly open = signal(false);
}

@Component({
  selector: 'apsi-palette',
  imports: [Icon, TPipe],
  template: `
    @if (state.open()) {
      <div class="palette-backdrop" (click)="close()"></div>
      <div class="palette" role="dialog" aria-modal="true" [attr.aria-label]="'palette.title' | t" (keydown)="onKey($event)">
        <div class="palette-search">
          <apsi-icon name="search" />
          <label for="palette-q" class="visually-hidden">{{ 'palette.title' | t }}</label>
          <input
            #box
            id="palette-q"
            type="search"
            role="combobox"
            aria-autocomplete="list"
            aria-controls="palette-list"
            [attr.aria-expanded]="results().length > 0"
            [attr.aria-activedescendant]="results()[active()] ? 'pi-' + active() : null"
            [placeholder]="'palette.placeholder' | t"
            [value]="query()"
            (input)="query.set($any($event.target).value); active.set(0)"
          />
          <kbd>Esc</kbd>
        </div>
        <ul id="palette-list" role="listbox" class="palette-list" [attr.aria-label]="'palette.results' | t">
          @for (item of results(); track item.id; let i = $index) {
            <li
              [id]="'pi-' + i"
              role="option"
              [attr.aria-selected]="i === active()"
              [class.active]="i === active()"
              (mouseenter)="active.set(i)"
              (click)="go(item)"
            >
              <apsi-icon [name]="item.icon" />
              <span class="palette-label">{{ item.label }}</span>
              <span class="palette-hint">{{ item.hint }}</span>
            </li>
          } @empty {
            <li class="palette-empty" role="presentation">{{ 'palette.none' | t }}</li>
          }
        </ul>
        <p class="palette-help muted">{{ 'palette.help' | t }}</p>
      </div>
    }
  `,
})
export class Palette {
  protected readonly state = inject(PaletteState);
  private readonly store = inject(CatalogStore);
  private readonly router = inject(Router);
  private readonly i18n = inject(I18n);
  private readonly box = viewChild<ElementRef<HTMLInputElement>>('box');
  readonly query = signal('');
  readonly active = signal(0);
  private returnFocus: HTMLElement | null = null;

  private readonly items = computed<PaletteItem[]>(() => {
    const pages = NAV_SECTIONS.flatMap((s) =>
      s.pages.map((p) => {
        const label = this.i18n.t(p.label);
        return { id: 'page:' + p.path, label, hint: this.i18n.t('palette.page'), icon: p.icon, commands: [p.path], haystack: label.toLowerCase() };
      }),
    );
    const domains = (this.store.catalog()?.domains ?? []).flatMap((d) => [
      { id: 'd:' + d.slug, label: d.name, hint: this.i18n.t('palette.domain'), icon: iconOf(d.slug), commands: ['/domains', d.slug], haystack: (d.name + ' ' + d.slug).toLowerCase() },
      ...d.entities.map((e) => ({
        id: 'e:' + d.slug + '/' + e.slug,
        label: e.name,
        hint: d.name,
        icon: iconOf(d.slug),
        commands: ['/domains', d.slug, e.slug],
        haystack: (e.name + ' ' + e.slug + ' ' + d.name).toLowerCase(),
      })),
    ]);
    return [...pages, ...domains];
  });

  readonly results = computed(() => {
    const terms = this.query().toLowerCase().trim().split(/\s+/).filter(Boolean);
    const all = this.items();
    const hits = terms.length === 0 ? all.filter((i) => i.id.startsWith('page:') || i.id.startsWith('d:')) : all.filter((i) => terms.every((t) => i.haystack.includes(t)));
    return hits.slice(0, 40);
  });

  constructor() {
    effect(() => {
      if (this.state.open()) {
        this.store.ensure();
        setTimeout(() => this.box()?.nativeElement.focus(), 0);
      }
    });
    document.addEventListener('keydown', (e) => {
      const target = e.target as HTMLElement | null;
      const typing = !!target && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));
      if ((e.key === 'k' && (e.ctrlKey || e.metaKey)) || (e.key === '/' && !typing && !this.state.open())) {
        e.preventDefault();
        this.show();
      }
    });
  }

  show(): void {
    this.returnFocus = document.activeElement as HTMLElement | null;
    this.query.set('');
    this.active.set(0);
    this.state.open.set(true);
  }

  close(): void {
    this.state.open.set(false);
    this.returnFocus?.focus();
  }

  go(item: PaletteItem): void {
    this.state.open.set(false);
    void this.router.navigate(item.commands);
  }

  onKey(e: KeyboardEvent): void {
    const n = this.results().length;
    if (e.key === 'Escape') {
      e.preventDefault();
      this.close();
    } else if (e.key === 'ArrowDown' && n) {
      e.preventDefault();
      this.active.update((a) => (a + 1) % n);
    } else if (e.key === 'ArrowUp' && n) {
      e.preventDefault();
      this.active.update((a) => (a - 1 + n) % n);
    } else if (e.key === 'Enter') {
      const item = this.results()[this.active()];
      if (item) {
        e.preventDefault();
        this.go(item);
      }
    } else if (e.key === 'Tab') {
      // keep focus inside the dialog
      e.preventDefault();
      this.box()?.nativeElement.focus();
    }
  }
}
