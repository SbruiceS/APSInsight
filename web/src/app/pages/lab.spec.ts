import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { Auth, authInterceptor } from '../core/auth';
import { LAB_EXAMPLES } from '../core/lab-examples';
import { LabPage } from './lab';

const envelope = (data: unknown) => ({ data, meta: { api_version: 'v1' } });
const FUNCTIONS = [
  { name: 'agro.gdd', description: 'Growing degree days (°C·day)', input: '{tmin, tmax, base, cap?}', output: 'number' },
  { name: 'stats.describe', description: 'n, mean, sd, quartiles', input: 'number[]', output: 'Descriptive' },
];

async function setup(fn?: string, input?: string) {
  TestBed.configureTestingModule({
    providers: [provideRouter([]), provideHttpClient(withInterceptors([authInterceptor])), provideHttpClientTesting()],
  });
  TestBed.inject(Auth).setToken('test-token-0000000000');
  const fixture = TestBed.createComponent(LabPage);
  if (fn) fixture.componentRef.setInput('fn', fn);
  if (input) fixture.componentRef.setInput('input', input);
  fixture.detectChanges();
  await fixture.whenStable();
  const http = TestBed.inject(HttpTestingController);
  http.expectOne('/api/v1/compute').flush(envelope(FUNCTIONS));
  fixture.detectChanges();
  await fixture.whenStable();
  fixture.detectChanges();
  return { fixture, http, el: fixture.nativeElement as HTMLElement };
}

const button = (el: HTMLElement, text: string) => Array.from(el.querySelectorAll('button')).find((b) => b.textContent?.trim().includes(text)) as HTMLButtonElement;

describe('LabPage', () => {
  afterEach(() => sessionStorage.clear());

  it('groups functions by family and prompts for a choice', async () => {
    const { el } = await setup();
    expect(el.textContent).toContain('Agronomy');
    expect(el.textContent).toContain('Statistics');
    expect(el.textContent).toContain('Pick a function from the list to begin.');
  });

  it('prefills from ?fn= and ?input=, runs, and shows a scalar result', async () => {
    const { fixture, http, el } = await setup('agro.gdd', '{"tmin":12,"tmax":31,"base":10}');
    const area = el.querySelector('#lab-input') as HTMLTextAreaElement;
    expect(JSON.parse(area.value)).toEqual({ tmin: 12, tmax: 31, base: 10 });
    button(el, 'Run').click();
    const req = http.expectOne('/api/v1/compute/agro.gdd');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual({ tmin: 12, tmax: 31, base: 10 });
    req.flush(envelope(11.5));
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
    expect(el.querySelector('.result-big')?.textContent).toContain('11.5');
    expect(el.textContent).toContain('Recent runs');
  });

  it('blocks invalid JSON before sending and shows server errors', async () => {
    const { fixture, http, el } = await setup('stats.describe');
    fixture.componentInstance.text.set('[1, 2,');
    fixture.detectChanges();
    expect(button(el, 'Run').disabled).toBe(true);
    expect(el.textContent).toContain('Invalid JSON');
    fixture.componentInstance.text.set('[]');
    fixture.detectChanges();
    button(el, 'Run').click();
    http
      .expectOne('/api/v1/compute/stats.describe')
      .flush({ error: { code: 'invalid_input', message: 'need at least one value', retryable: false }, meta: {} }, { status: 400, statusText: 'Bad Request' });
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
    expect(el.textContent).toContain('need at least one value');
  });

  it('has a synthetic example for every science registry function', () => {
    const registry = [
      'indices.registry', 'indices.compute', 'indices.summarize',
      'geo.distance', 'geo.area', 'geo.contains', 'geo.bbox', 'geo.web_mercator',
      'stats.mean', 'stats.describe', 'stats.mean_ci', 'stats.welch_t_test', 'stats.benjamini_hochberg', 'stats.bonferroni', 'stats.linear_regression', 'stats.forecast_errors',
      'ts.gaps', 'ts.late_arrivals', 'ts.downsample',
      'agro.gdd', 'agro.et0', 'agro.water_balance', 'agro.usda_texture', 'agro.moisture_correction', 'agro.convert', 'units.convert', 'agro.bushels_to_t_ha',
      'econ.margin', 'econ.emissions',
    ];
    expect(Object.keys(LAB_EXAMPLES).sort()).toEqual([...registry].sort());
  });

  it('labels every form control', async () => {
    const { el } = await setup('agro.gdd');
    for (const c of Array.from(el.querySelectorAll('input, select, textarea'))) {
      const id = c.getAttribute('id');
      expect((id && el.querySelector(`label[for="${id}"]`)) || c.closest('label'), c.outerHTML).toBeTruthy();
    }
  });
});
