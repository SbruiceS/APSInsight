import { Injectable, Pipe, PipeTransform, inject, signal } from '@angular/core';

// Localization-ready labels. Every key must exist in every catalogue;
// scripts/check-ui.mjs and i18n.spec.ts enforce parity.
export const EN = {
  'app.title': 'APSInsight',
  'app.skip': 'Skip to main content',
  'app.signOut': 'Sign out',
  'app.language': 'Language',
  'nav.label': 'Main navigation',
  'nav.overview': 'Overview',
  'nav.features': 'Feature register',
  'nav.notifications': 'Notifications',
  'nav.audit': 'Audit log',
  'login.title': 'Sign in',
  'login.token': 'API token',
  'login.help': 'Paste the bearer token issued by your administrator. It is kept for this browser session only.',
  'login.submit': 'Sign in',
  'login.failed': 'The token was not accepted.',
  'state.loading': 'Loading…',
  'state.empty': 'Nothing here yet.',
  'state.error': 'Something went wrong.',
  'state.denied': 'You do not have permission to view this.',
  'state.retry': 'Retry',
  'overview.title': 'Overview',
  'overview.summary': '{records} records across {domains} domains',
  'overview.filter': 'Filter domains',
  'overview.records': 'Records',
  'overview.entityKinds': 'Entity kinds',
  'overview.auditOk': 'Audit chain verified',
  'overview.auditBroken': 'Audit chain broken',
  'domain.entities': 'Entity kinds',
  'domain.activity': 'Recent activity',
  'domain.total': 'Total',
  'domain.active': 'Active',
  'domain.draft': 'Draft',
  'domain.archived': 'Archived',
  'domain.validity': 'Validity',
  'domain.completeness': 'Completeness',
  'records.search': 'Search records',
  'records.lifecycle': 'Lifecycle',
  'records.any': 'Any',
  'records.includeArchived': 'Include archived',
  'records.name': 'Name',
  'records.version': 'Version',
  'records.updated': 'Updated',
  'records.tags': 'Tags',
  'records.new': 'New record',
  'records.value': 'Value',
  'records.unit': 'Unit',
  'records.description': 'Description',
  'records.create': 'Create',
  'records.created': 'Record created.',
  'records.previous': 'Previous',
  'records.next': 'Next',
  'records.range': '{from}–{to} of {total}',
  'records.export': 'Export',
  'records.quality': 'Quality',
  'record.attributes': 'Attributes',
  'record.provenance': 'Provenance',
  'record.history': 'History',
  'record.archive': 'Archive',
  'record.restore': 'Restore',
  'record.archived': 'Record archived.',
  'record.restored': 'Record restored.',
  'record.conflict': 'This record changed since you loaded it. It has been reloaded.',
  'record.diff': 'Changes from previous version',
  'record.rollback': 'Restore this version',
  'record.legalHold': 'Under legal hold',
  'features.title': 'Feature register',
  'features.filterDomain': 'Domain',
  'features.filterCapability': 'Capability',
  'features.all': 'All',
  'features.query': 'Search requirements',
  'features.evidence': 'Evidence',
  'features.tests': 'Tests',
  'features.api': 'API',
  'features.engine': 'Engine',
  'features.requirement': 'Requirement',
  'features.status': 'Status',
  'features.gap': 'Known gap',
  'status.tested': 'Implemented and tested',
  'status.partial': 'Implemented but not fully tested',
  'status.specified': 'Specified only',
  'status.blocked': 'Blocked by an external dependency',
  'notifications.title': 'Notifications',
  'audit.title': 'Audit log',
  'audit.outcome': 'Outcome',
  'audit.action': 'Action',
  'audit.actor': 'Actor',
  'audit.target': 'Target',
  'audit.at': 'Time',
} as const;

export type I18nKey = keyof typeof EN;

export const HI: Record<I18nKey, string> = {
  'app.title': 'APSInsight',
  'app.skip': 'मुख्य सामग्री पर जाएँ',
  'app.signOut': 'साइन आउट',
  'app.language': 'भाषा',
  'nav.label': 'मुख्य नेविगेशन',
  'nav.overview': 'अवलोकन',
  'nav.features': 'फ़ीचर रजिस्टर',
  'nav.notifications': 'सूचनाएँ',
  'nav.audit': 'ऑडिट लॉग',
  'login.title': 'साइन इन',
  'login.token': 'API टोकन',
  'login.help': 'अपने व्यवस्थापक द्वारा जारी बेयरर टोकन चिपकाएँ। यह केवल इस ब्राउज़र सत्र के लिए रखा जाता है।',
  'login.submit': 'साइन इन करें',
  'login.failed': 'टोकन स्वीकार नहीं किया गया।',
  'state.loading': 'लोड हो रहा है…',
  'state.empty': 'अभी यहाँ कुछ नहीं है।',
  'state.error': 'कुछ गलत हो गया।',
  'state.denied': 'आपको इसे देखने की अनुमति नहीं है।',
  'state.retry': 'फिर से प्रयास करें',
  'overview.title': 'अवलोकन',
  'overview.summary': '{domains} डोमेन में {records} रिकॉर्ड',
  'overview.filter': 'डोमेन फ़िल्टर करें',
  'overview.records': 'रिकॉर्ड',
  'overview.entityKinds': 'इकाई प्रकार',
  'overview.auditOk': 'ऑडिट श्रृंखला सत्यापित',
  'overview.auditBroken': 'ऑडिट श्रृंखला टूटी हुई',
  'domain.entities': 'इकाई प्रकार',
  'domain.activity': 'हाल की गतिविधि',
  'domain.total': 'कुल',
  'domain.active': 'सक्रिय',
  'domain.draft': 'मसौदा',
  'domain.archived': 'संग्रहीत',
  'domain.validity': 'वैधता',
  'domain.completeness': 'पूर्णता',
  'records.search': 'रिकॉर्ड खोजें',
  'records.lifecycle': 'जीवनचक्र',
  'records.any': 'कोई भी',
  'records.includeArchived': 'संग्रहीत शामिल करें',
  'records.name': 'नाम',
  'records.version': 'संस्करण',
  'records.updated': 'अद्यतन',
  'records.tags': 'टैग',
  'records.new': 'नया रिकॉर्ड',
  'records.value': 'मान',
  'records.unit': 'इकाई',
  'records.description': 'विवरण',
  'records.create': 'बनाएँ',
  'records.created': 'रिकॉर्ड बनाया गया।',
  'records.previous': 'पिछला',
  'records.next': 'अगला',
  'records.range': '{total} में से {from}–{to}',
  'records.export': 'निर्यात',
  'records.quality': 'गुणवत्ता',
  'record.attributes': 'गुण',
  'record.provenance': 'उद्गम',
  'record.history': 'इतिहास',
  'record.archive': 'संग्रहीत करें',
  'record.restore': 'पुनर्स्थापित करें',
  'record.archived': 'रिकॉर्ड संग्रहीत किया गया।',
  'record.restored': 'रिकॉर्ड पुनर्स्थापित किया गया।',
  'record.conflict': 'लोड करने के बाद यह रिकॉर्ड बदल गया है। इसे फिर से लोड किया गया है।',
  'record.diff': 'पिछले संस्करण से परिवर्तन',
  'record.rollback': 'यह संस्करण पुनर्स्थापित करें',
  'record.legalHold': 'कानूनी रोक के अधीन',
  'features.title': 'फ़ीचर रजिस्टर',
  'features.filterDomain': 'डोमेन',
  'features.filterCapability': 'क्षमता',
  'features.all': 'सभी',
  'features.query': 'आवश्यकताएँ खोजें',
  'features.evidence': 'साक्ष्य',
  'features.tests': 'परीक्षण',
  'features.api': 'API',
  'features.engine': 'इंजन',
  'features.requirement': 'आवश्यकता',
  'features.status': 'स्थिति',
  'features.gap': 'ज्ञात कमी',
  'status.tested': 'लागू और परीक्षित',
  'status.partial': 'लागू, पर पूर्ण परीक्षण नहीं',
  'status.specified': 'केवल विनिर्दिष्ट',
  'status.blocked': 'बाहरी निर्भरता से अवरुद्ध',
  'notifications.title': 'सूचनाएँ',
  'audit.title': 'ऑडिट लॉग',
  'audit.outcome': 'परिणाम',
  'audit.action': 'क्रिया',
  'audit.actor': 'कर्ता',
  'audit.target': 'लक्ष्य',
  'audit.at': 'समय',
};

export const CATALOGUES: Record<string, Record<I18nKey, string>> = { en: EN, hi: HI };
export const LOCALES = [
  { code: 'en', label: 'English' },
  { code: 'hi', label: 'हिन्दी' },
];

@Injectable({ providedIn: 'root' })
export class I18n {
  readonly locale = signal(this.initial());

  private initial(): string {
    try {
      const saved = localStorage.getItem('apsi.locale');
      if (saved && saved in CATALOGUES) return saved;
    } catch {
      // storage unavailable: fall back to the default locale
    }
    return 'en';
  }

  setLocale(code: string): void {
    if (!(code in CATALOGUES)) return;
    this.locale.set(code);
    document.documentElement.lang = code;
    try {
      localStorage.setItem('apsi.locale', code);
    } catch {
      // ignore: preference simply won't persist
    }
  }

  t(key: I18nKey, params: Record<string, string | number> = {}): string {
    const text = CATALOGUES[this.locale()]?.[key] ?? EN[key];
    return text.replace(/\{(\w+)\}/g, (_, name: string) => String(params[name] ?? `{${name}}`));
  }

  number(value: number, digits = 0): string {
    return new Intl.NumberFormat(this.locale(), { maximumFractionDigits: digits }).format(value);
  }

  date(iso: string): string {
    return new Intl.DateTimeFormat(this.locale(), { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(iso));
  }
}

/** `{{ 'nav.overview' | t }}` — impure so it follows locale changes. */
@Pipe({ name: 't', pure: false })
export class TPipe implements PipeTransform {
  private readonly i18n = inject(I18n);
  transform(key: I18nKey, params?: Record<string, string | number>): string {
    return this.i18n.t(key, params);
  }
}
