.PHONY: build frontend-build run test test-integration lint

frontend-build:
	npm run build --prefix web

build: frontend-build
	cargo build --release --manifest-path server/Cargo.toml

run: frontend-build
	cargo run --manifest-path server/Cargo.toml

test:
	cargo test --manifest-path server/Cargo.toml
	npm test --prefix web

# 集成测试会清空指定库，并使用假发布凭据验证服务端遮罩。
test-integration:
ifndef AGENTSECRET_TEST_DATABASE_URL
	$(error AGENTSECRET_TEST_DATABASE_URL is required)
endif
ifndef TEST_RELEASE_CREDENTIAL
	$(error TEST_RELEASE_CREDENTIAL is required)
endif
	cargo test --manifest-path server/Cargo.toml --features integration

lint:
	cargo fmt --check --manifest-path server/Cargo.toml
	cargo clippy --all-targets --all-features --manifest-path server/Cargo.toml -- -D warnings
	npm run lint --prefix web
