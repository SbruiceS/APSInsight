.PHONY: help setup test lint

help:
	@echo "APSInsight development commands"
	@echo "  make setup     - Initial setup (install deps)"
	@echo "  make test      - Run all tests"
	@echo "  make lint      - Run linters"

setup:
	cd frontend && npm install
	cd backend/rails && bundle install
	cd python && pip install -e ".[dev]"
	# C++: cmake -S native -B native/build && cmake --build native/build

test:
	cd backend/rails && bundle exec rspec || true
	cd python && pytest || true
	cd frontend && npm test || true

lint:
	cd frontend && npm run lint || true
