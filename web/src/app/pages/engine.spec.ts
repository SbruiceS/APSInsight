import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { Auth, authInterceptor } from '../core/auth';
import { ENGINE_EXAMPLES, EnginePage } from './engine';

const envelope = (data: unknown) => ({ data, meta: { api_version: 'v1' } });
const CAPS = {
  engine_version: '0.1.0',
  backends: [
    { kind: 'cpu', available: true, detail: 'Rust, Rayon', kernels: ['ndvi'] },
    { kind: 'native', available: true, detail: 'C++17 kernels, ABI v1', kernels: ['gemm'] },
    { kind: 'accelerator', available: false, detail: 'AgriCUDA: planned ASPF product; CPU fallback is used', kernels: ['index.ndvi'] },
  ],
  operators: ['scan'],
  kernels: ['ndvi'],
  aggregates: ['mean'],
  sql: { engine: 'Apache DataFusion 54', columnar_format: 'Apache Arrow 58', export: ['json', 'parquet'], read_only: true },
  limits: { max_nodes: 256, max_memory_mb: 16384, max_output_rows: 100000, checkpoint_capacity_bytes: 1 },
};
const TABLES = [{ table: 'soil__ph', domain: 'soil', entity: 'ph', rows: 3, columns: [{ name: 'value', type: 'float64', unit: '1' }] }];

async function setup(table?: string) {
  TestBed.configureTestingModule({
    providers: [provideRouter([]), provideHttpClient(withInterceptors([authInterceptor])), provideHttpClientTesting()],
  });
  TestBed.inject(Auth).setToken('test-token-0000000000');
  const fixture = TestBed.createComponent(EnginePage);
  if (table) fixture.componentRef.setInput('table', table);
  fixture.detectChanges();
  await fixture.whenStable();
  const http = TestBed.inject(HttpTestingController);
  http.expectOne('/api/v1/engine').flush(envelope(CAPS));
  http.expectOne('/api/v1/engine/tables').flush(envelope(TABLES));
  await settle(fixture);
  return { fixture, http, el: fixture.nativeElement as HTMLElement };
}

async function settle(fixture: { detectChanges(): void; whenStable(): Promise<unknown> }) {
  fixture.detectChanges();
  await fixture.whenStable();
  fixture.detectChanges();
}

const button = (el: HTMLElement, text: string) => Array.from(el.querySelectorAll('button')).find((b) => b.textContent?.trim().includes(text)) as HTMLButtonElement;

describe('EnginePage', () => {
  afterEach(() => sessionStorage.clear());

  it('reports backend availability honestly', async () => {
    const { el } = await setup();
    const cards = el.querySelectorAll('.integration-card');
    expect(cards.length).toBe(3);
    expect(cards[2].getAttribute('data-state')).toBe('unavailable');
    expect(cards[2].textContent).toContain('CPU fallback');
    expect(el.textContent).toContain('Apache DataFusion 54');
  });

  it('prefills a query from ?table= and renders unit-annotated results', async () => {
    const { fixture, http, el } = await setup('soil__ph');
    expect((el.querySelector('#sql-text') as HTMLTextAreaElement).value).toContain('FROM soil__ph');
    button(el, 'Run').click();
    const req = http.expectOne('/api/v1/engine/sql');
    expect(req.request.body.sql).toContain('soil__ph');
    expect(req.request.body.max_rows).toBe(500);
    req.flush(envelope({ table: { columns: [{ name: 'value', type: 'float64', unit: '1' }], rows: [[6.4], [null]], total_rows: 2, truncated: false }, tables_used: ['soil__ph'], elapsed_us: 1500 }));
    await settle(fixture);
    const cells = Array.from(el.querySelectorAll('tbody td')).map((c) => c.textContent?.trim());
    expect(cells).toEqual(['6.4', '—']);
    expect(el.querySelector('thead th')?.textContent).toContain('(1)');
  });

  it('runs a workflow and shows rewrites, profile and failed checks', async () => {
    const { fixture, http, el } = await setup();
    button(el, 'Workflow plan').click();
    await settle(fixture);
    button(el, 'Run').click();
    const req = http.expectOne('/api/v1/engine/run');
    expect(req.request.body.outputs).toEqual(['by_plot', 'veg']);
    req.flush(
      envelope({
        plan: { optimized: { nodes: [], outputs: [] }, outputs: {}, rewrites: [{ rule: 'predicate_pushdown', detail: 'moved' }], levels: [['a']], costs: [{ id: 'veg', op: 'kernel', rows: 5, bytes: 1, work: 1, backend: 'cpu', reason: 'small' }], notes: [] },
        outputs: { veg: { columns: [{ name: 'ndvi', type: 'float64', unit: '1' }], rows: [[0.6]], total_rows: 1, truncated: false } },
        profile: [{ id: 'veg', op: 'kernel', backend: 'cpu', rows: 5, bytes: 80, elapsed_us: 20, cache: 'miss', note: '1 pixel(s) undefined' }],
        checks: [{ output: 'veg', column: 'ndvi', rule: '>= -1, <= 1', violations: 2, examples: [1, 2], passed: false }],
        memory: { limit_bytes: 1048576, peak_bytes: 2048, checkpoint_entries: 1, checkpoint_bytes: 80 },
        elapsed_us: 900,
      }),
    );
    await settle(fixture);
    expect(el.textContent).toContain('predicate_pushdown');
    expect(el.textContent).toContain('1 pixel(s) undefined');
    expect(el.textContent).toContain('2 violation(s)');
  });

  it('ships valid example workflows and labels every control', async () => {
    for (const x of ENGINE_EXAMPLES) {
      const wf = x.workflow as { nodes: { id: string }[]; outputs: string[] };
      const ids = wf.nodes.map((n) => n.id);
      expect(wf.outputs.every((o) => ids.includes(o)), x.id).toBe(true);
    }
    const { fixture, el } = await setup();
    for (const tab of ['SQL workbench', 'Workflow plan']) {
      button(el, tab).click();
      await settle(fixture);
      for (const c of Array.from(el.querySelectorAll('input, select, textarea'))) {
        const id = c.getAttribute('id');
        expect((id && el.querySelector(`label[for="${id}"]`)) || c.closest('label'), c.outerHTML).toBeTruthy();
      }
    }
  });
});
