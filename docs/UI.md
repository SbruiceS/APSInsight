# UI (`web/`, Angular 22)

```sh
cd web && npm ci
npm start        # dev server on :4200, proxies /api to 127.0.0.1:8080 (run apsi-server --dev)
npm run ci       # check-ui + unit tests (Vitest/jsdom) + production build
```

## Design

An agricultural design system in `src/styles.scss` and its partials in `src/styles/`:

- **Palette tokens** (`_tokens.scss`). Leaf, soil, wheat, sky, clay, berry and slate, each with a light and a dark value. The theme follows the OS, and the top-bar toggle (`core/theme.ts`) can force light or dark through `<html data-theme>`.
- **Tones** (`_tones.scss`). `data-tone`, `data-status`, `data-lifecycle`, `data-outcome`, `data-level`, `data-kind` and `data-state` all map to one `--tone` pair, so feature status, lifecycle and audit outcome are coloured consistently. Colour is always paired with text or an icon.
- **Animated icons** (`shared/icon.ts`, `shared/icons.ts`, `_icons.scss`). There are 60+ inline SVG icons, including agricultural ones: sprout, wheat, tractor, soil, drop, sprinkler, seed, barn and field. Each part has its own motion, such as swaying leaves, rolling wheels, falling rain or a turning gauge needle. Motion runs while the icon is `live`, or while the surrounding link, button or card is hovered or focused. `scripts/check-ui.mjs` fails if a motion has no keyframes.
- **Farm landscape** (`shared/widgets.ts`). A decorative scene with drifting clouds, swaying crops and a tractor, used on the sign-in and overview pages.
- **Reduced motion.** `prefers-reduced-motion: reduce` switches every animation off.

## Taxonomy

`core/taxonomy.ts` groups the 100 register domains into 14 agricultural areas, such as *Soil, water and irrigation* and *Geospatial and remote sensing*. Each area has an icon and a tone, and individual domains can override the icon. `check-ui.mjs` fails if a domain in `register/catalog.json` is missing from the taxonomy or listed twice.

## Views

| Route | View |
|---|---|
| `/login` | paste an API token (kept in `sessionStorage` for this browser session) |
| `/` | overview: KPI tiles (records, domains, entity kinds, audit chain, % tested), quick actions, agricultural areas, most active domains, latest notifications |
| `/domains?group=&q=` | all domains as cards, filtered by area chips and search; the filter is kept in the URL |
| `/domains/:domain` | **dashboard**: per-entity totals, lifecycle bar, validity and completeness meters, feature status for the domain, recent activity |
| `/domains/:domain/:entity` | records workspace: search, lifecycle filter, pagination, create form with unit suggestions and a validation report, export (CSV/JSON/GeoJSON), quality meters with formulas, on-demand descriptive statistics, hand-off to the Science Lab |
| `/records/:id` | record detail: attribute cards, provenance timeline, history with per-version diff and rollback, archive and restore, "Convert units" hand-off to the lab |
| `/lab?fn=&input=` | **Science Lab**: every compute function grouped by family (indices, agronomy, units, geo, stats, time series, economics). Includes a JSON editor with validation, synthetic worked examples for all 29 functions, result cards and raw JSON, and recent runs. The URL can be shared |
| `/status` | honest completion: donut of the four statuses, a stacked bar per agricultural area, and a per-domain table filtered by status, linking to the feature register |
| `/features?domain=&q=` | feature register browser: filter by domain or capability, full-text search, evidence per feature with status and known gap |
| `/integrations` | ASPF products: live availability, contract version, operations, link to contract features; all optional |
| `/notifications` | timeline of rule-driven alerts linking to records |
| `/audit` | audit log with on-demand hash-chain verification, text and outcome filters (auditor or admin) |

## Interconnection

- **Command palette.** Press Ctrl/Cmd + K or `/`. It searches every page, domain and entity kind.
- **Deep links.** Domain cards link to entity records. Status rows, domain dashboards and integration cards link to `/features?domain=`. Records and record pages pass data to `/lab?fn=…&input=…`. Audit targets that name a record link to it.
- **One-time catalogue fetch.** The catalogue is fetched once (`core/catalog-store.ts`) and shared, so breadcrumbs, the palette and the feature register show human names without extra requests.
- **Toasts.** Toasts confirm actions, and the same text is announced through the polite live region.

Every data view renders explicit **loading**, **empty**, **error** (with retry) and **permission-denied** states through `apsi-state`. Version conflicts reload the record and announce why.

## Accessibility

- A skip link, landmark `nav` and `main`, and `aria-current` on the active navigation item. On narrow screens the sidebar collapses behind a labelled menu button.
- Focus moves to the page heading after each navigation. The command palette is a modal dialog with a combobox and listbox (`aria-activedescendant`); it traps Tab and restores focus when it closes.
- One polite `aria-live` status region for create, export, archive, restore, rollback, lab runs and conflict messages.
- Every form control is labelled (enforced by `scripts/check-ui.mjs`, `records.spec.ts` and `lab.spec.ts`). Filter chips use `aria-pressed`.
- Icons are `aria-hidden` unless they carry meaning on their own. Charts (donut, stacked bars, lifecycle bars) have text equivalents, legends or `aria-label`s.
- Tables have captions and `th` scopes.
- Visible `:focus-visible` outlines, light and dark themes, and `prefers-reduced-motion` support.

## Localization

All labels come from `src/app/core/i18n.ts`, in English and Hindi. Parity of keys and `{placeholder}` names is enforced by `i18n.spec.ts` and `check-ui.mjs`. Numbers and dates are formatted with `Intl` for the active locale, and `<html lang>` follows the selected language.
