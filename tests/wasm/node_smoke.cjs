const assert = require("assert");
const fs = require("fs");
const path = require("path");

function repoRoot() {
  return path.resolve(__dirname, "..", "..");
}

function readHexFile(p) {
  const s = fs.readFileSync(p, "utf8").trim();
  return Buffer.from(s, "hex");
}

function readBytesFile(p) {
  return fs.readFileSync(p);
}

function loadWasmPkg(relPkgDir, jsEntry) {
  const root = repoRoot();
  const entry = path.join(root, relPkgDir, jsEntry);
  // eslint-disable-next-line global-require, import/no-dynamic-require
  return require(entry);
}

function mustThrowContains(fn, substr) {
  let threw = false;
  try {
    fn();
  } catch (e) {
    threw = true;
    const msg = String(e && e.message ? e.message : e);
    assert(
      msg.includes(substr),
      `expected error to include '${substr}', got: ${msg}`
    );
  }
  assert(threw, `expected throw containing '${substr}', but did not throw`);
}

function toPlainJs(v) {
  if (v instanceof Map) {
    const o = {};
    for (const [k, vv] of v.entries()) {
      o[k] = toPlainJs(vv);
    }
    return o;
  }
  if (Array.isArray(v)) {
    return v.map(toPlainJs);
  }
  return v;
}

function main() {
  const root = repoRoot();
  const vectorsDir = path.join(root, "tests", "vectors", "capsule");

  const ublCapsuleWasm = loadWasmPkg(
    path.join("target", "wasm-pkg", "ubl-capsule-wasm"),
    "ubl_capsule_wasm.js"
  );
  const aiNrf1Wasm = loadWasmPkg(
    path.join("target", "wasm-pkg", "ai-nrf1-wasm"),
    "ai_nrf1_wasm.js"
  );

  // ai-nrf1 wasm sanity: encode -> decode -> verify
  const encoded = aiNrf1Wasm.encode({ a: 1, b: true, c: "b3:deadbeef" });
  assert(encoded instanceof Uint8Array, "encode must return Uint8Array");
  assert.strictEqual(aiNrf1Wasm.verify(encoded), true);
  const decoded = toPlainJs(aiNrf1Wasm.decode(encoded));
  assert.deepStrictEqual(decoded, { a: 1, b: true, c: "b3:deadbeef" });

  const alicePk = readHexFile(path.join(vectorsDir, "alice.pk"));
  const keyring = JSON.parse(
    fs.readFileSync(path.join(vectorsDir, "keyring.json"), "utf8")
  );

  // Positive: seal verify (bytes)
  for (const name of ["capsule_ack", "capsule_ask", "capsule_nack"]) {
    const capBytes = readBytesFile(
      path.join(vectorsDir, `${name}.signed.nrf`)
    );
    ublCapsuleWasm.verifySealBytes(capBytes, alicePk, 0n);
  }

  // Positive: receipts chain verify (bytes + keyring)
  for (const name of ["capsule_ack.chain2", "capsule_ask.chain2"]) {
    const capBytes = readBytesFile(
      path.join(vectorsDir, `${name}.signed.nrf`)
    );
    ublCapsuleWasm.verifySealBytes(capBytes, alicePk, 0n);
    const rep = ublCapsuleWasm.verifyReceiptsChainBytes(capBytes, keyring);
    assert.strictEqual(rep.ok, true);
    assert.strictEqual(rep.hops, 2);
  }

  // Negative: expired should fail without skew
  const expiredBytes = readBytesFile(
    path.join(vectorsDir, "capsule_expired.signed.nrf")
  );
  mustThrowContains(
    () => ublCapsuleWasm.verifySealBytes(expiredBytes, alicePk, 0n),
    "Err.Hdr.Expired"
  );

  // Negative: skew vector fails without skew; succeeds with huge skew
  const skewBytes = readBytesFile(
    path.join(vectorsDir, "capsule_expired_skew.signed.nrf")
  );
  mustThrowContains(
    () => ublCapsuleWasm.verifySealBytes(skewBytes, alicePk, 0n),
    "Err.Hdr.Expired"
  );
  ublCapsuleWasm.verifySealBytes(skewBytes, alicePk, 9223372036854775807n);

  // Negative: tampered (signed but mutated) should fail with IdMismatch
  const tamperedBytes = readBytesFile(
    path.join(vectorsDir, "capsule_ack.tampered.signed.nrf")
  );
  mustThrowContains(
    () => ublCapsuleWasm.verifySealBytes(tamperedBytes, alicePk, 0n),
    "Err.Seal.IdMismatch"
  );

  console.log("OK");
}

main();
