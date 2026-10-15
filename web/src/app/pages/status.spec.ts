import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { Auth, authInterceptor } from '../core/auth';
import { StatusPage } from './status';

const envelope = (data: unknown) => ({ data, meta: { api_version: 'v1' } });
const counts = (t: number, p: number, s: number, b: number) => ({
  'implemented-and-tested': t,
  'implemented-not-fully-tested': p,
  'specified-only': s,
  'blocked-external': b,
});
const SUMMARY = {
  total: 62,
  by_status: [
    { status: 'implemented-and-tested', label: 'implemented and tested', features: 30 },
    { status: 'implemented-not-fully-tested', label: 'implemented, not fully tested', features: 10 },
    { status: 'specified-only', label: 'specified only', features: 1 },
    { status: 'blocked-external', label: 'blocked by external dependency', features: 21 },
  ],
  domains: [
    { domain: 'field-and-farm-model', name: 'Field and Farm Model', counts: counts(30, 1, 0, 0) },
    { domain: 'future-agriql-integration-contract', name: 'Future AgriQL Integration Contract', counts: counts(0, 9, 1, 21) },
  ],
};

describe('StatusPage', () => {
  afterEach(() => sessionStorage.clear());

  it('shows honest totals as text and filters domains by status', async () => {
    TestBed.configureTestingModule({
      providers: [provideRouter([]), provideHttpClient(withInterceptors([authInterceptor])), provideHttpClientTesting()],
    });
    TestBed.inject(Auth).setToken('test-token-0000000000');
    const fixture = TestBed.createComponent(StatusPage);
    fixture.detectChanges();
    TestBed.inject(HttpTestingController).expectOne('/api/v1/status').flush(envelope(SUMMARY));
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;

    // Every status count is written out in the legend, not only drawn.
    const legend = el.querySelector('.legend')?.textContent ?? '';
    expect(legend).toContain('Implemented and tested: 30');
    expect(legend).toContain('Blocked by an external dependency: 21');
    expect(el.querySelector('.donut-total')?.textContent).toContain('48.4%');
    expect(el.querySelectorAll('tbody tr').length).toBe(2);

    const blocked = Array.from(el.querySelectorAll('.chip')).find((c) => c.textContent?.includes('Blocked')) as HTMLButtonElement;
    blocked.click();
    fixture.detectChanges();
    expect(blocked.getAttribute('aria-pressed')).toBe('true');
    const rows = el.querySelectorAll('tbody tr');
    expect(rows.length).toBe(1);
    expect(rows[0].querySelector('a')?.getAttribute('href')).toBe('/features?domain=future-agriql-integration-contract');
  });
});
