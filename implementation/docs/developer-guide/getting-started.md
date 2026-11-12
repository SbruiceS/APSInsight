# Development Guide

## Prerequisites

- Ruby 3.3 + Bundler
- PostgreSQL 16
- Node.js 20+
- Python 3.11+
- CMake 3.25+ and a C++20 compiler (for native)

## Backend (Rails)

```bash
cd backend/rails
bundle install
# Create database
createdb apsinsight_development
bundle exec rails db:migrate
bundle exec rails db:seed
bundle exec rails server
```

API base: http://localhost:3000/api/v1

## Frontend

```bash
cd frontend
npm install
npm run dev
```

Set `VITE_API_BASE_URL=http://localhost:3000/api/v1` in `.env` if needed.

## Python

```bash
cd python
pip install -e ".[dev]"
pytest
```

## C++ Native

```bash
cd native
cmake -S . -B build
cmake --build build
ctest --test-dir build
```

## Background Jobs

Sidekiq is configured. Start a worker with:

```bash
cd backend/rails
bundle exec sidekiq
```

Imports are enqueued automatically on `POST /api/v1/imports`.

## AgriQL (Database Query Language)

AgriQL is fully wired as the agricultural query language.

- Frontend workspace: http://localhost:5173/agriql
- API: `POST /api/v1/agriql/query` `{ "sql": "SHOW VERSION;" }`
- Real execution requires the AgriQL Python package (`pip install` from SbruiceS/AgriQL bindings).
- Without it the adapter returns an explicit test-double result (never invented data).

See `integrations/agriql/README.md` for the supported grammar subset.

## Agricultural Analytics

`GET /api/v1/datasets/:id/analytics/soil_summary` returns a structured soil nutrient summary with explicit assumptions.
