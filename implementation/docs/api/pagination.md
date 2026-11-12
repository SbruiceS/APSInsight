# Pagination

List endpoints currently return the full collection (no cursor or page parameters).

Exception:
- `GET /api/v1/observations` is hard-limited to the most recent 500 rows.

Future plan: `?page=` and `?per_page=` (or cursor) on all index actions, with `Link` headers or a `meta` object containing `total`, `page`, `per_page`.
