# Accessibility checklist (WCAG 2.2 AA)

Columns: **Auto** is checked by `web/scripts/check-ui.mjs`, Angular AOT checks or unit tests on every `verify`. **Manual** must be checked by a person before a release and recorded below.

| # | Check | Auto | Manual |
|---|---|---|---|
| 1 | Every form control has an accessible label | ✓ (labelled controls) | screen reader spot check |
| 2 | Status and error messages are announced (`aria-live`) | ✓ | NVDA/VoiceOver: loading, empty, error, permission-denied states |
| 3 | Skip link to main content | ✓ | – |
| 4 | Full keyboard operation: tab order, visible focus, no traps | – | ✓ every page |
| 5 | Status is never conveyed by colour alone (badges also carry text) | – | ✓ |
| 6 | Contrast ≥ 4.5:1 for text and ≥ 3:1 for UI components | – | ✓ (tooling such as axe DevTools) |
| 7 | Respects `prefers-reduced-motion` | – | ✓ |
| 8 | Layout reflows at 320 CSS px and 200% zoom without loss | – | ✓ |
| 9 | Language attribute set; every label translated (en, hi) | ✓ (catalogue completeness) | Hindi review by a fluent speaker |
| 10 | Tables have header cells; charts have a text alternative | – | ✓ |
| 11 | Focus moves to the new content after navigation | – | ✓ |
| 12 | Session timeouts warn before expiry | n/a (no timeout) | – |

## Plan

- **Automated now:** items 1–3 and 9 (`npm run check`, run by `tools/verify.mjs`).
- **To automate next:** an axe-core run over each route in a headless browser (needs a browser e2e harness, which does not exist yet).
- **Manual audit record:**

| Date | Release | Auditor | Items failed | Issues |
|---|---|---|---|---|
| – | – | – | not yet performed | – |

Until a manual audit is recorded, accessibility features are reported as "implemented but not fully tested" in [STATUS](../docs/STATUS.md).
