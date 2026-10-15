import { Component, computed, input } from '@angular/core';
import { ICONS, IconName, IconPart } from './icons';

/**
 * `<apsi-icon name="sprout" />` — an animated agricultural icon.
 *
 * Decorative by default (`aria-hidden`); pass `label` when the icon carries
 * meaning on its own. Parts animate while the enclosing link, button or card
 * is hovered or focused; `live` keeps the animation running (used for the
 * brand mark and loading states). Reduced-motion users see a static icon.
 */
@Component({
  selector: 'apsi-icon',
  host: { class: 'apsi-icon', '[class.live]': 'live()', '[attr.data-size]': 'size()' },
  template: `
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.75"
      stroke-linecap="round"
      stroke-linejoin="round"
      focusable="false"
      [attr.role]="label() ? 'img' : null"
      [attr.aria-label]="label() || null"
      [attr.aria-hidden]="label() ? null : 'true'"
    >
      @for (part of parts(); track $index) {
        <svg:g [attr.class]="part.motion ? 'm-' + part.motion : null">
          @for (d of part.d; track $index) {
            <svg:path [attr.d]="d" [attr.fill]="part.fill ? 'currentColor' : null" />
          }
        </svg:g>
      }
    </svg>
  `,
})
export class Icon {
  readonly name = input.required<IconName>();
  readonly label = input('');
  readonly live = input(false);
  readonly size = input<'sm' | 'md' | 'lg' | 'xl'>('md');
  readonly parts = computed<IconPart[]>(() => ICONS[this.name()]);
}
