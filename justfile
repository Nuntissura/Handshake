set dotenv-load := false

dev:
	cd app && pnpm run tauri dev

lint:
	cd app && pnpm run lint
	cd src/backend/handshake_core && cargo clippy --all-targets --all-features

test:
	cd src/backend/handshake_core && cargo test
