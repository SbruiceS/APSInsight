import { TestBed } from '@angular/core/testing';
import { CATALOGUES, EN, I18n, I18nKey } from './i18n';

describe('I18n', () => {
  it('every catalogue has exactly the English keys, all non-empty', () => {
    const keys = Object.keys(EN).sort();
    for (const [code, cat] of Object.entries(CATALOGUES)) {
      expect(Object.keys(cat).sort(), code).toEqual(keys);
      for (const k of keys) expect(cat[k as I18nKey].trim().length, `${code}:${k}`).toBeGreaterThan(0);
    }
  });

  it('placeholders match across catalogues', () => {
    const vars = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join(',');
    for (const cat of Object.values(CATALOGUES)) {
      for (const k of Object.keys(EN) as I18nKey[]) expect(vars(cat[k]), k).toBe(vars(EN[k]));
    }
  });

  it('interpolates and switches locale', () => {
    const i18n = TestBed.inject(I18n);
    i18n.setLocale('en');
    expect(i18n.t('records.range', { from: 1, to: 25, total: 300 })).toBe('1–25 of 300');
    i18n.setLocale('hi');
    expect(i18n.t('nav.overview')).toBe('अवलोकन');
    expect(document.documentElement.lang).toBe('hi');
    i18n.setLocale('xx');
    expect(i18n.locale()).toBe('hi');
    i18n.setLocale('en');
  });
});
