.PHONY: vectors vectors-keys vectors-gen vectors-sign vectors-verify
.PHONY: check-base

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
