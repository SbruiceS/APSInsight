import { Routes } from '@angular/router';
import { signedInGuard } from './core/auth';
import { AuditPage, NotificationsPage } from './pages/activity';
import { DomainPage } from './pages/domain';
import { FeaturesPage } from './pages/features';
import { LoginPage } from './pages/login';
import { OverviewPage } from './pages/overview';
import { RecordPage } from './pages/record';
import { RecordsPage } from './pages/records';

export const routes: Routes = [
  { path: 'login', component: LoginPage, title: 'Sign in · APSInsight' },
  {
    path: '',
    canActivate: [signedInGuard],
    children: [
      { path: '', component: OverviewPage, title: 'Overview · APSInsight' },
      { path: 'features', component: FeaturesPage, title: 'Feature register · APSInsight' },
      { path: 'notifications', component: NotificationsPage, title: 'Notifications · APSInsight' },
      { path: 'audit', component: AuditPage, title: 'Audit log · APSInsight' },
      { path: 'domains/:domain', component: DomainPage, title: 'Domain · APSInsight' },
      { path: 'domains/:domain/:entity', component: RecordsPage, title: 'Records · APSInsight' },
      { path: 'records/:id', component: RecordPage, title: 'Record · APSInsight' },
    ],
  },
  { path: '**', redirectTo: '' },
];
