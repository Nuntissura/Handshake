set dotenv-load := false

dev:
	cd app && npm run tauri dev

lint:
	cd app && npm run lint
	cd src/backend/handshake_core && cargo clippy --all-targets --all-features

test:
	cd src/backend/handshake_core && cargo test
