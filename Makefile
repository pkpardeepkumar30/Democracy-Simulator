.PHONY: run stop logs test reset

run:
	docker compose up --build -d

stop:
	docker compose down

logs:
	docker compose logs -f game

test:
	./scripts/smoke-test.sh

reset:
	docker compose down -v
