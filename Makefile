.PHONY: vectors vectors-keys vectors-gen vectors-sign vectors-verify
.PHONY: check-base
.PHONY: pm2-ai-start pm2-ai-stop pm2-ai-restart pm2-ai-logs
.PHONY: pm2-ai-tunnel-start pm2-ai-tunnel-stop pm2-ai-tunnel-restart pm2-ai-tunnel-logs

vectors: vectors-keys vectors-gen vectors-sign

vectors-keys:
	@bash tools/vectors/gen_keys.sh

vectors-gen:
	@bash tools/vectors/gen_vectors.sh

vectors-sign:
	@bash tools/vectors/sign_vectors.sh

vectors-verify:
	@bash tools/vectors/verify_vectors.sh

check-base:
	cargo fmt --all -- --check
	cargo clippy -p nrf-core -p ai-nrf1 -p ubl_json_view -p ubl_capsule --all-targets --all-features -- -D warnings
	cargo test --workspace --locked
	make vectors-verify

pm2-ai-start:
	cargo build --release -p registry
	./tools/pm2/pm2-ai start ops/pm2/ecosystem.config.cjs
	./tools/pm2/pm2-ai save

pm2-ai-stop:
	./tools/pm2/pm2-ai stop ai-nrf1-registry || true
	./tools/pm2/pm2-ai save || true

pm2-ai-restart:
	./tools/pm2/pm2-ai restart ai-nrf1-registry

pm2-ai-logs:
	./tools/pm2/pm2-ai logs ai-nrf1-registry

pm2-ai-tunnel-start:
	./tools/pm2/pm2-ai start ops/pm2/ecosystem.tunnel.config.cjs
	./tools/pm2/pm2-ai save

pm2-ai-tunnel-stop:
	./tools/pm2/pm2-ai stop ai-nrf1-tunnel || true
	./tools/pm2/pm2-ai save || true

pm2-ai-tunnel-restart:
	./tools/pm2/pm2-ai restart ai-nrf1-tunnel

pm2-ai-tunnel-logs:
	./tools/pm2/pm2-ai logs ai-nrf1-tunnel
