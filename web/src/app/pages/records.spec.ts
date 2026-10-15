import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { Auth, authInterceptor } from '../core/auth';
import { RecordsPage } from './records';

const BASE = '/api/v1/domains/soil-profile-and-classification/entities/ph';
const envelope = (data: unknown) => ({ data, meta: { api_version: 'v1' } });
const page = (items: unknown[]) => ({ total: items.length, offset: 0, limit: 25, items, facets: {} });
const record = {
  id: 'rec_0190f0000000700080000000000000aa',
  tenant: 't',
  domain: 'soil-profile-and-classification',
  entity: 'ph',
  schema_version: 1,
  version: 2,
  lifecycle: 'active',
  name: 'Plot 7 topsoil',
  attributes: { value: 6.4 },
  tags: ['north'],
  owner: 'eve',
  legal_hold: false,
  created_at: '2026-05-01T10:00:00Z',
  updated_at: '2026-05-02T10:00:00Z',
  created_by: 'eve',
  updated_by: 'eve',
  provenance: [],
};

async function setup() {
  TestBed.configureTestingModule({
    providers: [provideRouter([]), provideHttpClient(withInterceptors([authInterceptor])), provideHttpClientTesting()],
  });
  TestBed.inject(Auth).setToken('test-token-0000000000');
  const fixture = TestBed.createComponent(RecordsPage);
  fixture.componentRef.setInput('domain', 'soil-profile-and-classification');
  fixture.componentRef.setInput('entity', 'ph');
  fixture.detectChanges();
  await fixture.whenStable();
  return { fixture, http: TestBed.inject(HttpTestingController), el: fixture.nativeElement as HTMLElement };
}

async function flush(fixture: Awaited<ReturnType<typeof setup>>['fixture']) {
  fixture.detectChanges();
  await fixture.whenStable();
  fixture.detectChanges();
}

describe('RecordsPage', () => {
  afterEach(() => sessionStorage.clear());

  it('shows loading, then the empty state, and sends the bearer token', async () => {
    const { fixture, http, el } = await setup();
    expect(el.textContent).toContain('Loading');
    const req = http.expectOne((r) => r.url === `${BASE}/records`);
    expect(req.request.headers.get('Authorization')).toBe('Bearer test-token-0000000000');
    expect(req.request.params.get('limit')).toBe('25');
    req.flush(envelope(page([])));
    http.expectOne(`${BASE}/quality`).flush(envelope({ records: 0, completeness: null, validity: null, provenance_coverage: null, mean_freshness_days: null, formulas: {}, missing_data: '', limitations: '' }));
    await flush(fixture);
    expect(el.textContent).toContain('Nothing here yet.');
    http.verify();
  });

  it('renders rows with links to record detail', async () => {
    const { fixture, http, el } = await setup();
    http.expectOne((r) => r.url === `${BASE}/records`).flush(envelope(page([record])));
    http.expectOne(`${BASE}/quality`).flush(envelope({ records: 1, completeness: 0.2, validity: 1, provenance_coverage: 0, mean_freshness_days: 1, formulas: { completeness: 'f' }, missing_data: '', limitations: '' }));
    await flush(fixture);
    const link = el.querySelector('tbody a') as HTMLAnchorElement;
    expect(link.textContent).toBe('Plot 7 topsoil');
    expect(link.getAttribute('href')).toBe(`/records/${record.id}`);
    expect(el.textContent).toContain('1–1 of 1');
    expect(el.textContent).toContain('100%');
  });

  it('shows the permission-denied state on 403 and an error with retry on 500', async () => {
    const { fixture, http, el } = await setup();
    http.expectOne((r) => r.url === `${BASE}/records`).flush({ error: { code: 'forbidden', message: 'no', retryable: false }, meta: {} }, { status: 403, statusText: 'Forbidden' });
    http.expectOne(`${BASE}/quality`).flush({ error: { code: 'storage_unavailable', message: 'disk', retryable: true }, meta: {} }, { status: 503, statusText: 'Unavailable' });
    await flush(fixture);
    expect(el.textContent).toContain('You do not have permission to view this.');
    const retry = Array.from(el.querySelectorAll('button')).find((b) => b.textContent?.trim() === 'Retry');
    expect(retry).toBeTruthy();
    retry!.click();
    http.expectOne(`${BASE}/quality`);
  });

  it('labels every form control', async () => {
    const { el } = await setup();
    for (const c of Array.from(el.querySelectorAll('input, select, textarea'))) {
      const id = c.getAttribute('id');
      const labelled = (id && el.querySelector(`label[for="${id}"]`)) || c.closest('label');
      expect(labelled, c.outerHTML).toBeTruthy();
    }
  });
});
