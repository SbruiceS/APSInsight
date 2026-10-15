import { TestBed } from '@angular/core/testing';
import { GROUPS, groupOf, iconOf } from '../core/taxonomy';
import { Icon } from './icon';
import { ICONS, ICON_NAMES } from './icons';
import { Logo } from './logo';

describe('Icon', () => {
  it('is decorative by default and an image when labelled', () => {
    const fixture = TestBed.createComponent(Icon);
    fixture.componentRef.setInput('name', 'sprout');
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    const svg = el.querySelector('svg') as SVGElement;
    expect(svg.getAttribute('aria-hidden')).toBe('true');
    expect(svg.getAttribute('role')).toBeNull();
    expect(el.querySelectorAll('g[class^="m-"]').length).toBeGreaterThan(0);

    fixture.componentRef.setInput('label', 'Seedling');
    fixture.componentRef.setInput('live', true);
    fixture.detectChanges();
    expect(svg.getAttribute('role')).toBe('img');
    expect(svg.getAttribute('aria-label')).toBe('Seedling');
    expect(svg.getAttribute('aria-hidden')).toBeNull();
    expect(el.classList.contains('live')).toBe(true);
  });

  it('every icon has drawable paths and agricultural icons are animated', () => {
    for (const name of ICON_NAMES) {
      const parts = ICONS[name];
      expect(parts.length, name).toBeGreaterThan(0);
      for (const p of parts) for (const d of p.d) expect(d, name).toMatch(/^M/);
    }
    for (const name of ['sprout', 'leaf', 'wheat', 'tractor', 'drop', 'sun', 'soil', 'seed', 'sprinkler'] as const) {
      expect(
        ICONS[name].some((p) => p.motion),
        name,
      ).toBe(true);
    }
  });
});

describe('Logo', () => {
  it('is one labelled image and only animates on request', () => {
    const fixture = TestBed.createComponent(Logo);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    const svg = el.querySelector('svg') as SVGElement;
    expect(svg.getAttribute('role')).toBe('img');
    expect(svg.getAttribute('aria-label')).toBe('APSInsight');
    expect(el.classList.contains('animated')).toBe(false);
    expect(el.querySelector('text')?.textContent?.replace(/\s/g, '')).toBe('Insight');
    // A, P and S drawn once visibly and once more as the sheen clip.
    expect(el.querySelectorAll('.lg-letter').length).toBe(3);
    expect(el.querySelectorAll('clipPath path').length).toBe(3);

    fixture.componentRef.setInput('animated', true);
    fixture.detectChanges();
    expect(el.classList.contains('animated')).toBe(true);
  });
});

describe('taxonomy', () => {
  it('lists each domain once and resolves icons and groups', () => {
    const all = GROUPS.flatMap((g) => g.domains);
    expect(new Set(all).size).toBe(all.length);
    expect(all.length).toBe(100);
    expect(groupOf('soil-chemistry-and-fertility').id).toBe('soilWater');
    expect(groupOf('no-such-domain').id).toBe('other');
    for (const d of all) expect(ICON_NAMES, d).toContain(iconOf(d));
  });
});
