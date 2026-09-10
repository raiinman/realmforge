// SPDX-License-Identifier: AGPL-3.0-only
//
// Client-side BnetSRP6v2 — pure JavaScript, no dependencies.
//
// Implements the exact algorithm from the captured account.battle.net SRP
// flow: PBKDF2-HMAC-SHA-512 key derivation, BigInt modular arithmetic,
// SHA-256 evidence hashing. Uses the Web Crypto API (all modern browsers).
//
// Reference: Phoenix reverse-engineering spec, oauth-oidc-implementation.md,
// TrinityCore BnetSRP6v2Base::CalculateX.

/**
 * Parse a hex string into a big-endian byte array.
 */
function hexToBytes(hex) {
	const bytes = new Uint8Array(hex.length / 2);
	for (let i = 0; i < bytes.length; i++) {
		bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
	}
	return bytes;
}

/**
 * Big-endian byte array to uppercase hex string.
 */
function bytesToHex(bytes) {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, "0"))
		.join("")
		.toUpperCase();
}

/**
 * Big-endian byte array to BigInt (unsigned).
 */
function bytesToBigInt(bytes) {
	let hex = "";
	for (const b of bytes) {
		hex += b.toString(16).padStart(2, "0");
	}
	if (hex.length === 0) return 0n;
	return BigInt("0x" + hex);
}

/**
 * BigInt to fixed-width big-endian byte array.
 */
function bigIntToBytes(n, width) {
	let hex = n.toString(16);
	if (hex.length % 2 !== 0) hex = "0" + hex;
	const bytes = hexToBytes(hex);
	if (bytes.length > width) {
		throw new Error(
			`value requires ${bytes.length} bytes, exceeds width ${width}`,
		);
	}
	if (bytes.length < width) {
		const padded = new Uint8Array(width);
		padded.set(bytes, width - bytes.length);
		return padded;
	}
	return bytes;
}

/**
 * Signed big-endian encoding for evidence vectors: minimal bytes with a
 * leading 0x00 when the high bit of the first byte is set.
 *
 * This is the "broken evidence vector" encoding from the reference implementation
 * (GetBrokenEvidenceVector / jsbn toByteArray).
 */
function signedBytes(n) {
	let hex = n.toString(16);
	if (hex.length % 2 !== 0) hex = "0" + hex;
	const bytes = hexToBytes(hex);
	if (bytes.length === 0) return new Uint8Array([0]);
	if (bytes[0] & 0x80) {
		const padded = new Uint8Array(bytes.length + 1);
		padded.set(bytes, 1);
		return padded;
	}
	return bytes;
}

/**
 * Fixed-width big-endian: zero-padded to exactly `width` bytes.
 */
function padBytes(n, width) {
	return bigIntToBytes(n, width);
}

/**
 * SHA-256 hash of concatenated bytes.
 */
async function sha256(...arrays) {
	const totalLen = arrays.reduce((s, a) => s + a.length, 0);
	const combined = new Uint8Array(totalLen);
	let offset = 0;
	for (const a of arrays) {
		combined.set(a, offset);
		offset += a.length;
	}
	const hash = await crypto.subtle.digest("SHA-256", combined);
	return new Uint8Array(hash);
}

/**
 * Derived private exponent x = PBKDF2-HMAC-SHA-512(username:password, salt,
 * 15000 iterations, 64 bytes), interpreted as signed big-endian, reduced
 * modulo N-1.
 */
async function computeX(username, password, saltBytes, iterations) {
	const ikm = new TextEncoder().encode(username + ":" + password);

	const key = await crypto.subtle.importKey(
		"raw",
		ikm,
		{ name: "PBKDF2" },
		false,
		["deriveBits"],
	);

	const dk = await crypto.subtle.deriveBits(
		{
			name: "PBKDF2",
			salt: saltBytes,
			iterations: iterations,
			hash: "SHA-512",
		},
		key,
		512, // 64 bytes * 8 bits
	);

	const dkBytes = new Uint8Array(dk);
	// Signed big-endian: if high bit is set, treat as negative (two's complement).
	let x = bytesToBigInt(dkBytes);
	const twoTo512 = 1n << 512n;
	if (x >= twoTo512 / 2n) {
		x = x - twoTo512;
	}
	return x;
}

/**
 * Compute client SRP proof: publicA and clientEvidenceM1.
 *
 * Parameters:
 *   username  - SRP username from challenge (uppercase hex)
 *   password  - plaintext password
 *   saltHex   - salt from challenge (hex)
 *   publicBHex - server ephemeral B from challenge (hex)
 *   nHex      - modulus from challenge (hex)
 *   generator - generator from challenge (string, e.g. "2")
 *   iterations - PBKDF2 iterations from challenge
 *
 * Returns: {publicA: BigInt, clientEvidenceM1: BigInt}
 */
async function clientProof(
	username,
	password,
	saltHex,
	publicBHex,
	nHex,
	generator,
	iterations,
) {
	const N = BigInt("0x" + nHex);
	const g = BigInt(generator);
	const B = BigInt("0x" + publicBHex);
	const salt = hexToBytes(saltHex);

	const FIELD_WIDTH = 256; // 2048-bit modulus

	// x = derived private exponent
	const x = await computeX(username, password, salt, iterations);
	const xMod = ((x % (N - 1n)) + (N - 1n)) % (N - 1n);

	// Random client private ephemeral a (32 bytes)
	const aBytes = crypto.getRandomValues(new Uint8Array(32));
	const a = BigInt("0x" + bytesToHex(aBytes));

	// A = g^a mod N
	const A = modPow(g, a, N);

	// k = SHA256(pad256(N) || pad256(g))
	const kHash = await sha256(
		padBytes(N, FIELD_WIDTH),
		padBytes(g, FIELD_WIDTH),
	);
	const k = bytesToBigInt(kHash);

	// u = SHA256(pad256(A) || pad256(B))
	const uHash = await sha256(
		padBytes(A, FIELD_WIDTH),
		padBytes(B, FIELD_WIDTH),
	);
	const u = bytesToBigInt(uHash);

	// S = (B - k * g^x)^(a + u * x) mod N
	const gx = modPow(g, xMod, N);
	let base = (B - k * gx) % N;
	if (base < 0n) base += N;
	const exp = a + u * xMod;
	const S = modPow(base, exp, N);

	// M1 = SHA256(signed_be(A) || signed_be(B) || signed_be(S))
	const m1Hash = await sha256(signedBytes(A), signedBytes(B), signedBytes(S));
	const M1 = bytesToBigInt(m1Hash);

	return { publicA: A, clientEvidenceM1: M1 };
}

/**
 * Modular exponentiation: base^exp mod m.
 * Uses exponentiation by squaring.
 */
function modPow(base, exp, mod) {
	if (mod === 1n) return 0n;
	let result = 1n;
	base = base % mod;
	let e = exp;
	while (e > 0n) {
		if (e & 1n) {
			result = (result * base) % mod;
		}
		base = (base * base) % mod;
		e = e >> 1n;
	}
	return result;
}

// Export for use in templates and tests.
if (typeof module !== "undefined" && module.exports) {
	module.exports = {
		clientProof,
		hexToBytes,
		bytesToHex,
		bytesToBigInt,
		bigIntToBytes,
		signedBytes,
		padBytes,
		sha256,
		computeX,
		modPow,
	};
}
