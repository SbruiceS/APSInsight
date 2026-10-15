# UI (`web/`, Angular 22)

```sh
cd web && npm ci
npm start        # dev server on :4200, proxies /api to 127.0.0.1:8080 (run apsi-server --dev)
npm run ci       # check-ui + unit tests (Vitest/jsdom) + production build
```

## Views

| Route | View |
|---|---|
| `/login` | paste an API token (kept in `sessionStorage` for this browser session) |
| `/` | overview: records per domain, audit-chain status, domain filter |
| `/domains/:domain` | **dashboard**: per-entity totals, lifecycle counts, validity and completeness, each linking down to its records; recent activity |
| `/domains/:domain/:entity` | records workspace: search, lifecycle filter, pagination, create form with a validation report, export (CSV/JSON/GeoJSON), quality indicators with formulas |
| `/records/:id` | record detail: attributes, provenance, history with per-version diff and rollback, archive and restore |
| `/features` | feature register browser: filter by domain or capability, full-text search, evidence per feature |
| `/notifications` | outbox |
| `/audit` | audit log (auditor or admin) |

Every data view renders explicit **loading**, **empty**, **error** (with retry) and **permission-denied** states through `apsi-state`. Version conflicts reload the record and announce why.

## Accessibility

- A skip link, landmark `nav` and `main`, and `aria-current` on the active navigation item.
- Focus moves to the page heading after each navigation.
- One polite `aria-live` status region for create, archive, restore and conflict messages.
- Every form control is labelled (enforced by `scripts/check-ui.mjs` and `records.spec.ts`).
- Tables have captions and `th` scopes.
- Visible `:focus-visible` outlines, `prefers-color-scheme` dark theme, and `prefers-reduced-motion` support.

## Localization

All labels come from `src/app/core/i18n.ts`, in English and Hindi. Parity of keys and `{placeholder}` names is enforced by `i18n.spec.ts` and `check-ui.mjs`. Numbers and dates are formatted with `Intl` for the active locale, and `<html lang>` follows the selected language.
