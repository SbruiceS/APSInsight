import { Routes } from '@angular/router';
import { signedInGuard } from './core/auth';

// Every page is lazy-loaded so the initial bundle carries only the shell.
export const routes: Routes = [
  { path: 'login', loadComponent: () => import('./pages/login').then((m) => m.LoginPage), title: 'Sign in · APSInsight' },
  {
    path: '',
    canActivate: [signedInGuard],
    children: [
      { path: '', loadComponent: () => import('./pages/overview').then((m) => m.OverviewPage), title: 'Overview · APSInsight' },
      { path: 'domains', loadComponent: () => import('./pages/domains').then((m) => m.DomainsPage), title: 'Domains · APSInsight' },
      { path: 'domains/:domain', loadComponent: () => import('./pages/domain').then((m) => m.DomainPage), title: 'Domain · APSInsight' },
      { path: 'domains/:domain/:entity', loadComponent: () => import('./pages/records').then((m) => m.RecordsPage), title: 'Records · APSInsight' },
      { path: 'records/:id', loadComponent: () => import('./pages/record').then((m) => m.RecordPage), title: 'Record · APSInsight' },
      { path: 'lab', loadComponent: () => import('./pages/lab').then((m) => m.LabPage), title: 'Science lab · APSInsight' },
      { path: 'status', loadComponent: () => import('./pages/status').then((m) => m.StatusPage), title: 'Feature status · APSInsight' },
      { path: 'features', loadComponent: () => import('./pages/features').then((m) => m.FeaturesPage), title: 'Feature register · APSInsight' },
      { path: 'integrations', loadComponent: () => import('./pages/integrations').then((m) => m.IntegrationsPage), title: 'ASPF integrations · APSInsight' },
      { path: 'notifications', loadComponent: () => import('./pages/activity').then((m) => m.NotificationsPage), title: 'Notifications · APSInsight' },
      { path: 'audit', loadComponent: () => import('./pages/activity').then((m) => m.AuditPage), title: 'Audit log · APSInsight' },
    ],
  },
  { path: '**', redirectTo: '' },
];
