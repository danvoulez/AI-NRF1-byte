.PHONY: vectors vectors-keys vectors-gen vectors-sign vectors-verify

vectors: vectors-keys vectors-gen vectors-sign

vectors-keys:
	@bash tools/vectors/gen_keys.sh

vectors-gen:
	@bash tools/vectors/gen_vectors.sh

vectors-sign:
	@bash tools/vectors/sign_vectors.sh

vectors-verify:
	@bash tools/vectors/verify_vectors.sh

