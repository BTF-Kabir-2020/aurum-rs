RUST_LOG ?= info

.PHONY: fmt fmt-check check clippy test gates demo doctor smoke docker-up docker-down backup-roundtrip clean

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

check:
	cargo check --all-targets --all-features

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all-targets --all-features

gates: fmt-check check clippy test
	@echo "ALL QUALITY GATES PASSED"

demo:
	AURUM_MODE=demo cargo run -- demo

doctor:
	AURUM_MODE=demo cargo run -- doctor

smoke: gates
	AURUM_MODE=demo cargo run -- doctor
	AURUM_DB_PATH=/tmp/aurum-smoke.db AURUM_MODE=demo cargo run -- backup create --output /tmp/aurum-smoke.bak
	AURUM_DB_PATH=/tmp/aurum-smoke.db AURUM_MODE=demo cargo run -- backup verify /tmp/aurum-smoke.bak
	@echo "SMOKE PASSED"

backup-roundtrip:
	AURUM_DB_PATH=/tmp/aurum-roundtrip.db AURUM_MODE=demo cargo run -- backup create --output /tmp/aurum-roundtrip.bak
	AURUM_DB_PATH=/tmp/aurum-roundtrip.db AURUM_MODE=demo cargo run -- backup verify /tmp/aurum-roundtrip.bak
	AURUM_DB_PATH=/tmp/aurum-restored.db AURUM_MODE=demo cargo run -- backup restore /tmp/aurum-roundtrip.bak

docker-up:
	docker compose up -d --build

docker-down:
	docker compose down

clean:
	cargo clean
	rm -f /tmp/aurum-smoke.db /tmp/aurum-smoke.bak /tmp/aurum-roundtrip.db /tmp/aurum-roundtrip.bak /tmp/aurum-restored.db