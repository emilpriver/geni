test:
	docker compose up -d
	cargo test
	docker compose down -v
