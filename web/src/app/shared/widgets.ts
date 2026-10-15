import { Component, computed, input } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Tone } from '../core/taxonomy';
import { Icon } from './icon';
import { IconName } from './icons';

/**
 * Horizontal meter for a 0..1 ratio. The value is always written as text next
 * to the bar, so it never relies on colour or length alone. `null` (no data)
 * renders as "—", never as 0%.
 */
@Component({
  selector: 'apsi-meter',
  template: `
    <span class="meter-label">{{ label() }}</span>
    <span class="meter-track" aria-hidden="true"><span class="meter-fill" [style.width.%]="pct() ?? 0"></span></span>
    <span class="meter-value">{{ pct() === null ? '—' : pct() + '%' }}</span>
  `,
  host: { class: 'meter', '[attr.data-level]': 'level()' },
})
export class Meter {
  readonly label = input.required<string>();
  readonly value = input<number | null>(null);
  readonly pct = computed(() => {
    const v = this.value();
    return v === null ? null : Math.round(Math.min(1, Math.max(0, v)) * 1000) / 10;
  });
  readonly level = computed(() => {
    const p = this.pct();
    if (p === null) return 'none';
    if (p >= 90) return 'good';
    if (p >= 60) return 'fair';
    return 'poor';
  });
}

/** KPI tile with an animated icon; becomes a link when `link` is set. */
@Component({
  selector: 'apsi-stat',
  imports: [Icon, RouterLink],
  template: `
    @if (link(); as l) {
      <a class="stat-tile" [routerLink]="l" [attr.data-tone]="tone()">
        <apsi-icon [name]="icon()" size="lg" />
        <span class="stat-value">{{ value() }}</span>
        <span class="stat-label">{{ label() }}</span>
        @if (hint()) {
          <span class="stat-hint">{{ hint() }}</span>
        }
      </a>
    } @else {
      <div class="stat-tile" [attr.data-tone]="tone()">
        <apsi-icon [name]="icon()" size="lg" />
        <span class="stat-value">{{ value() }}</span>
        <span class="stat-label">{{ label() }}</span>
        @if (hint()) {
          <span class="stat-hint">{{ hint() }}</span>
        }
      </div>
    }
  `,
})
export class Stat {
  readonly icon = input.required<IconName>();
  readonly label = input.required<string>();
  readonly value = input.required<string>();
  readonly hint = input('');
  readonly tone = input<Tone>('leaf');
  readonly link = input<string | unknown[] | null>(null);
}

export interface DonutSlice {
  label: string;
  value: number;
  tone: string;
}

/** SVG donut chart. The legend repeats every value as text, and each slice also has a pattern so colour is not the only cue. */
@Component({
  selector: 'apsi-donut',
  template: `
    <figure class="donut">
      <svg viewBox="0 0 42 42" role="img" [attr.aria-label]="caption()">
        <circle class="donut-hole" cx="21" cy="21" r="15.915" />
        @for (s of arcs(); track s.label) {
          <circle
            class="donut-arc"
            cx="21"
            cy="21"
            r="15.915"
            [attr.data-tone]="s.tone"
            [attr.stroke-dasharray]="s.dash"
            [attr.stroke-dashoffset]="s.offset"
          />
        }
        <text x="21" y="20" class="donut-total">{{ centre() }}</text>
        <text x="21" y="25.5" class="donut-sub">{{ centreLabel() }}</text>
      </svg>
      <figcaption>
        <ul class="legend">
          @for (s of slices(); track s.label) {
            <li><span class="swatch" [attr.data-tone]="s.tone" aria-hidden="true"></span>{{ s.label }}: <strong>{{ s.value }}</strong></li>
          }
        </ul>
      </figcaption>
    </figure>
  `,
})
export class Donut {
  readonly slices = input.required<DonutSlice[]>();
  readonly caption = input.required<string>();
  readonly centre = input('');
  readonly centreLabel = input('');
  readonly arcs = computed(() => {
    const total = this.slices().reduce((a, s) => a + s.value, 0) || 1;
    let acc = 0;
    return this.slices().map((s) => {
      const pct = (s.value / total) * 100;
      const arc = { label: s.label, tone: s.tone, dash: `${pct} ${100 - pct}`, offset: `${25 - acc}` };
      acc += pct;
      return arc;
    });
  });
}

/**
 * Animated farm landscape (sun, drifting clouds, swaying crop rows, a tractor
 * crossing the field). Purely decorative; motion stops under reduced motion.
 */
@Component({
  selector: 'apsi-landscape',
  template: `
    <svg class="landscape" viewBox="0 0 640 220" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id="ls-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" class="ls-sky-top" />
          <stop offset="1" class="ls-sky-bottom" />
        </linearGradient>
      </defs>
      <rect width="640" height="220" fill="url(#ls-sky)" />
      <g class="ls-sun"><circle cx="520" cy="58" r="26" /></g>
      <g class="ls-cloud ls-cloud-a"><path d="M80 60h70a18 18 0 0 0 0-36 26 26 0 0 0-49 6 15 15 0 0 0-21 30z" /></g>
      <g class="ls-cloud ls-cloud-b"><path d="M300 40h52a13 13 0 0 0 0-26 19 19 0 0 0-36 4 11 11 0 0 0-16 22z" /></g>
      <path class="ls-hill-far" d="M0 140C120 100 220 120 320 110s220-40 320 0v110H0z" />
      <path class="ls-hill" d="M0 160c140-30 260-20 360-10s200 0 280-20v90H0z" />
      <g class="ls-rows">
        @for (x of rows; track x) {
          <g class="ls-crop" [style.--d]="(x % 7) * 0.15 + 's'">
            <path [attr.d]="'M' + x + ' 210v-22'" />
            <path [attr.d]="'M' + x + ' 196c-6 0-9-3-9-9 6 0 9 3 9 9zM' + x + ' 192c0-6 3-9 9-9 0 6-3 9-9 9z'" />
          </g>
        }
      </g>
      <g class="ls-tractor">
        <path d="M0 0h22l6 10h14v10H0z" />
        <path d="M6 0v-12h12v12" />
        <circle cx="10" cy="22" r="8" class="ls-wheel" />
        <circle cx="38" cy="24" r="5" class="ls-wheel" />
      </g>
    </svg>
  `,
})
export class Landscape {
  protected readonly rows = Array.from({ length: 28 }, (_, i) => 12 + i * 23);
}
