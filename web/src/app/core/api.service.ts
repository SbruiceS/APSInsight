import { HttpClient, HttpErrorResponse, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable, catchError, map, throwError } from 'rxjs';
import {
  ApsiRecord,
  AuditEvent,
  Catalog,
  DomainDashboard,
  Envelope,
  ErrorBody,
  Feature,
  FieldChange,
  Lifecycle,
  NewRecord,
  Notification,
  Overview,
  Page,
  Principal,
  QualityReport,
  RecordVersion,
  SearchPage,
  TraceRow,
} from './models';

/** Normalised API failure used by every view's error state. */
export class ApiFailure extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
    readonly retryable: boolean,
    readonly details?: unknown,
  ) {
    super(message);
  }

  get denied(): boolean {
    return this.status === 403;
  }

  static from(e: unknown): ApiFailure {
    if (e instanceof ApiFailure) return e;
    if (e instanceof HttpErrorResponse) {
      const body = (e.error as { error?: ErrorBody } | null)?.error;
      if (body) return new ApiFailure(e.status, body.code, body.message, body.retryable, body.details);
      return new ApiFailure(e.status, e.status === 0 ? 'network' : 'http_error', e.message, e.status === 0 || e.status >= 500);
    }
    return new ApiFailure(0, 'unknown', String(e), false);
  }
}

export interface ListQuery {
  q?: string;
  lifecycle?: Lifecycle | '';
  includeArchived?: boolean;
  offset?: number;
  limit?: number;
  facets?: string[];
}

@Injectable({ providedIn: 'root' })
export class Api {
  private readonly http = inject(HttpClient);

  private unwrap<T>(obs: Observable<Envelope<T>>): Observable<T> {
    return obs.pipe(
      map((e) => e.data),
      catchError((e: unknown) => throwError(() => ApiFailure.from(e))),
    );
  }

  private get<T>(url: string, params?: HttpParams): Observable<T> {
    return this.unwrap(this.http.get<Envelope<T>>(url, { params }));
  }

  private post<T>(url: string, body: unknown): Observable<T> {
    return this.unwrap(this.http.post<Envelope<T>>(url, body));
  }

  me(): Observable<Principal> {
    return this.get('/api/v1/me');
  }

  catalog(): Observable<Catalog> {
    return this.get('/api/v1/catalog');
  }

  overview(): Observable<Overview> {
    return this.get('/api/v1/overview');
  }

  dashboard(domain: string): Observable<DomainDashboard> {
    return this.get(`/api/v1/domains/${encodeURIComponent(domain)}/dashboard`);
  }

  records(domain: string, entity: string, q: ListQuery): Observable<SearchPage> {
    let params = new HttpParams().set('offset', q.offset ?? 0).set('limit', q.limit ?? 25).set('sort', 'updated_at').set('desc', true);
    if (q.q) params = params.set('q', q.q);
    if (q.lifecycle) params = params.set('lifecycle', q.lifecycle);
    if (q.includeArchived) params = params.set('include_archived', true);
    if (q.facets?.length) params = params.set('facets', q.facets.join(','));
    return this.get(`${this.kind(domain, entity)}/records`, params);
  }

  create(domain: string, entity: string, body: NewRecord): Observable<ApsiRecord> {
    return this.post(`${this.kind(domain, entity)}/records`, body);
  }

  quality(domain: string, entity: string): Observable<QualityReport> {
    return this.get(`${this.kind(domain, entity)}/quality`);
  }

  exportFile(domain: string, entity: string, format: 'json' | 'csv' | 'geojson'): Observable<Blob> {
    return this.http
      .get(`${this.kind(domain, entity)}/export`, { params: new HttpParams().set('format', format), responseType: 'blob' })
      .pipe(catchError((e: unknown) => throwError(() => ApiFailure.from(e))));
  }

  record(id: string): Observable<ApsiRecord> {
    return this.get(`/api/v1/records/${encodeURIComponent(id)}`);
  }

  transition(id: string, to: Lifecycle, expectedVersion: number): Observable<ApsiRecord> {
    return this.post(`/api/v1/records/${encodeURIComponent(id)}/lifecycle`, { to, expected_version: expectedVersion });
  }

  history(id: string): Observable<RecordVersion[]> {
    return this.get(`/api/v1/records/${encodeURIComponent(id)}/history`);
  }

  diff(id: string, from: number, to: number): Observable<FieldChange[]> {
    return this.get(`/api/v1/records/${encodeURIComponent(id)}/diff`, new HttpParams().set('from', from).set('to', to));
  }

  rollback(id: string, toVersion: number, expectedVersion: number): Observable<ApsiRecord> {
    return this.post(`/api/v1/records/${encodeURIComponent(id)}/rollback`, { to_version: toVersion, expected_version: expectedVersion });
  }

  features(filter: { domain?: string; capability?: string; q?: string; offset: number; limit: number }): Observable<Page<Feature>> {
    let params = new HttpParams().set('offset', filter.offset).set('limit', filter.limit);
    if (filter.domain) params = params.set('domain', filter.domain);
    if (filter.capability) params = params.set('capability', filter.capability);
    if (filter.q) params = params.set('q', filter.q);
    return this.get('/api/v1/features', params);
  }

  feature(id: string): Observable<{ feature: Feature; evidence: TraceRow }> {
    return this.get(`/api/v1/features/${encodeURIComponent(id)}`);
  }

  notifications(): Observable<Notification[]> {
    return this.get('/api/v1/notifications');
  }

  audit(limit = 200): Observable<AuditEvent[]> {
    return this.get('/api/v1/audit', new HttpParams().set('limit', limit));
  }

  private kind(domain: string, entity: string): string {
    return `/api/v1/domains/${encodeURIComponent(domain)}/entities/${encodeURIComponent(entity)}`;
  }
}
