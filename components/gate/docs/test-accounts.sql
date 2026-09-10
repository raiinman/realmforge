-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Test accounts from the TrinityCore srp_reference tool.
-- Fixed salt (32 zero bytes), BnetSRP6v2 verifiers, 15000 iterations.
-- See docs/test-accounts.md for passwords.
--

-- 'test@example.com' (password: 'password')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1001, 'test@example.com', 'USA', 'Tester#1001', 'Test', 'User', '1988-03-14')
ON CONFLICT (email) DO UPDATE SET id = 1001;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1001,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('89bb000decf06db12d4e826f8e85d077bd8e9119f1639c4c6c22a5897d8554b34a7a3d1ca81882c0934d11a93de20713b73e3f657e07e7e16c3cdd8fe46ab902ca769687753f5ae1452f788f7c7beeece22af4d5869d4019388e5b0e9e25de1f09808d869a71ff3ad2573a6b1f55a04116cb95f51da0e65f9d10ee61e1ead6df25df4c2bd8ceb3053a37e5bbec209014d59919314f1403a3bd0bb28d8c609821dd397eaac2db4ef72ac403fc5da1dcdc3282845bc4835dc9bd3cd13bc3337a5e851e5f3a231e2c5a18144840e5c9fc780892e3e488b4875edf1b9355ebd72c46c8dc74f2b0fb1696596a0b98f198800abf3afd2d63b9b010ec7f1bc16671f914','hex'),
  15000, 2,
  '6a8d11e65b26a5272f3b5220d528b2f4df73bf6e264bab1f45c1eab3ad9890b0')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1001, '1001#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'admin@bnet.local' (password: '123456')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1002, 'admin@bnet.local', 'DEU', 'Tester#1002', 'Admin', 'Benetz', '1990-06-21')
ON CONFLICT (email) DO UPDATE SET id = 1002;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1002,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('68746f71a376815fe590d92430cd2a98ef39997a27b496483617f183ff193be5b57590471a510f55cdfd0aa24c58c37e06e465f6466a6f400d522f145060e3b6cacd111f89282d90446492f30cf7acca823d4219436421ae236eeb37cfaf15622da2a9e5c633abe237b4834a484848866c49dc79b15156110950a8790c84ea0ab33c85e23be905c8f65992a9b26e76d9c4a04ea60d020316edc062185b2872dffef854b7692aa6a4fc6d7832aa40c056e49c088bae2a05b65838a09002007bf26764ee530746789453fe7c4a235b6dd67cf1ef85f1c3844f4929b3bbb363f064f8d433512e6329e614c9e6710b65523be9e117925ee5b8faa8ed2c3fb187e9c4','hex'),
  15000, 2,
  '5f9935df08cf791ef7c6441f68d57de892413b60ec9f67b06cbcb08c1b44de8e')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1002, '1002#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'player@example.org' (password: 'Correct Horse Battery Staple')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1003, 'player@example.org', 'USA', 'Tester#1003', 'Player', 'One', '1991-11-02')
ON CONFLICT (email) DO UPDATE SET id = 1003;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1003,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('a548aae2c2c61c8e169009c4684929e2723971764c165f2ddc335242222e3f304beb46ff603890d235ed95979c9398c32491382ed2e6ff9c2334e1ec559570240255275fe6b3775dae3dbd18f5fad933b80f9b308e4ee74f7f733d38a5601abe3019d894fc876ceac9430a5ef5550001c54ede5392361cc0f67ae50c24f381170bef8df100fc611199795061768b3868177c5d3ff2d85c5c9ce1371611121d0b9feb5b6d794760d80768795eed4c1f1c58dd99413a05fea9c5171ada31a4b55f939420776d8c162deda789949d8b3dbbad6fd96aa2371f78c3e86cf0b3941b8321fba6e417df0c25612d4fcf2bf5b581f58cc6ba496bf4ff98b9b5445632fb6b','hex'),
  15000, 2,
  '8eb26dd3cc39ebbe2201d2b79afad62be92cb4a98d4b271c8a30deb021012b91')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1003, '1003#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'user@example.com' (password: 'MixedCasePassword')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1004, 'user@example.com', 'KOR', 'Tester#1004', 'Min', 'Kim', '1992-01-15')
ON CONFLICT (email) DO UPDATE SET id = 1004;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1004,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('6a3c2c73a3791ce3f09444069a6e8f8cf2c7c1ca641cfff2dd6bf3df638a776752779136580218729fee091167714b8523232f1215a01a56eb29362f6f3607a22868ded7b0c9665733d1f1c061cfa6dcb22dea7aa8a7b19fc419adcf8bddd85b5968fac959c83a81534b9ed86a7d4dcebd8be8c7d4eb6ff3bf9c81f33980f3875039134c7fb4d4c36584b5eb241d78e4a1f8c8672800f1f7efdbde502f32030e8ade512d1563ea304d3ff5f6a0528ae093be73f2d1314abfd112b14b4732e315404fb3e9118f43bd6937d925c0a622dc178f79772e4b9bc12fe47577317f8bba70d74ccc03f28f2f5959da69a5f493a823b251e9acfd7c522b445d8fa3c02f09','hex'),
  15000, 2,
  '8c0f041b1cd8eae1cc3328b298caaab1e0fdd39e52756498ffb31cdfbe47a17e')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1004, '1004#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'guest@bnet.local' (password: 'p@ssw0rd!#$%^&*()')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1005, 'guest@bnet.local', 'USA', 'Tester#1005', 'Guest', 'Suspended', '1985-09-30')
ON CONFLICT (email) DO UPDATE SET id = 1005;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1005,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('9802c91db7ca42d5a1f6ae12016fcad04a5d40972c5b1d828f1fb33db6e269399cf147299a278279acb041a4335b3135867ee986cbaa9af40af51a1a01eed979488f702ecfcd70388941a0b879175391352e972c84a6f0d07e2372e786527829313616a6f8030cef20ffc1e12e675d5945c51711c7c3dcb010e99086dc73aa91d4e4277194815f3388fbff06737df72f9b0eb3e653353a6c424eeb0d78973b272450a97d6902bbda3569da76c03e9d65ffe0357aaba2e4664013633abe4aa87fcdbbebc4f2148ea89ea264d5c2efe5f5512fbc3ded86890a72a8d9b82292b728ed4ec841e1ba215b837ffe84ae6f69cf00672031b88436fc52c38dbc99f76919','hex'),
  15000, 2,
  '495cea93234580158ba4cff3baedb4c75beb70b25dcbdc6759e70415d37f26fb')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1005, '1005#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'a@b.cd' (password: 'x')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1006, 'a@b.cd', 'GBR', 'Tester#1006', 'Expired', 'User', '1993-04-11')
ON CONFLICT (email) DO UPDATE SET id = 1006;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1006,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('3ea8e373c510a5b88d21c03401903404f12413de54a93063ea9727b8e28fbc0446671f7eb740b59677d2304fb5f61cd597a304984a4f322407574e7206b1a5f41414e5305e2d79e6397c82ae9b9de4b79bf945c2da0e4ce6479d782bc6c3ef3b9546d84af1478789c43241ac2b370bea6c248942a4a0f224d09228094c318dddfea8cb9e85b6152ac83a125e67c6ed898abbe4e65b77ad2f0954609d155c901cf8e5412c3a68fc26e24ad64553e05f3a0ea31aae78d3fe2945b81b18aa460a32d73ad8c508e4f446b5b40064bd8c009cbf89c611ea18e50550f6912bb919d84c590416141d35d91acc9cd413719adfcceaf54cad4d24e29ed81c4d821e7063d4','hex'),
  15000, 2,
  '35aebc0846f72f441a081d9183fa2ced9826f735e0060a45d3fb71866b5704ef')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1006, '1006#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'empty@example.com' (password: '')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1007, 'empty@example.com', 'CHN', 'Tester#1007', 'Trial', 'User', '1994-07-19')
ON CONFLICT (email) DO UPDATE SET id = 1007;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1007,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('2ef3c7a6606802bd4e4581d785e4fc2f10ace5891731d45b1ab4bc7d5771f6d8ceacaf000222daac27560388022aecb0ddbd605b32f99a28f53011719a64872b44491fbc3a04466fb80981c2fc81dff06c087df6f723d32d191ff644a99fe820eb588ec31d13838bb9fd6492ab9f53abbd75a561397ea4bb7a642ae44ba9c9c9a20232706890d532893d2f5d5ae0fdbda2699a029a781d6456f5f19cbb8b837ad31b054daad8ea01f7340e8dd6dfbbc51135f581c93a70e80c81693feee87f25771928cc8c192e9a62c002a889af928b7d6642a217fbbfbca07f5ce641b239b8ddc11c9b26490093fcead6073a74bf0d57e853fc10a3fbf78a1ba1b628031a82','hex'),
  15000, 2,
  '740d8d6358d37469fb940bb595127d6ff321a7f6594b7ace4636c0b0d3543eed')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1007, '1007#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'numeric@example.com' (password: '01234567890123456789')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1008, 'numeric@example.com', 'USA', 'Tester#1008', 'Numeric', 'Banned', '1987-12-05')
ON CONFLICT (email) DO UPDATE SET id = 1008;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1008,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('3c0301373205f0bd3dd9a248460df014f8cb900d2ec455f08a3eb22d5c3eba5d82911e085a49297d1b2cbb5cc73e21b35cad6c4394d69d590ab464c1173a7fe128abe11c16a99a974149a9207defe720238139e861da34561351c763509262c3e5060918e76b604b51701429ce98e2c0768358b67c428b93bff19119518d8b2a8de5ad94834ec6e40cc3f09bd8a997a2b5f6863437e04d61e0eec0f68193c9fff02e93e8e5e6e52fcbaf61b290fa8abfa3bfde8f5080347ac7df2418d5d86de0db0ec7f70354e2692556b52aab96e40b9ec99164ab42abc6e044673a5531e34d3c7262e08b491c29f0f181e798aac0a8734082db190f237afa006d71961f52cb','hex'),
  15000, 2,
  'fb3ab027594dee19c0c3ee8dc432317dba62b714560b30a09a9bd107aa91cf4a')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1008, '1008#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'utf8@example.com' (password: 'üñîçøé')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1009, 'utf8@example.com', 'TWN', 'Tester#1009', 'Classic', 'User', '1995-02-23')
ON CONFLICT (email) DO UPDATE SET id = 1009;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1009,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('30dc1f2866bd942960d1c91faf45bb1c95b02e62321a0d897d4273e8a7f43cb8a9be0e4b208029458d5cffb907779d6872961d1607f946ce3dfe71de3f6f33c47aee6efdf473ecf3446e55bc0f35ff874610bc4817305fc26d60faf004f03f79baa7e4eee353bdae692f8c00b91f6d38ea4b328be7c72dabcfa3751cb4f1a6237966532c12640fbb7031acfc8f600d18348cd8125a65b83e663aef483e8af90f49b28c930550394d331cef48162150b627a9899320803e19788d10cd919ef5ad73bd1c27f4faa5f6c50231e8148f2e0b656233e360ba3902c1fa2f1213be9840baa903a245d0fb41f4945c5be59a112d09b9bfe3ea1004d1b66dc82133e0cba4','hex'),
  15000, 2,
  '94a4f8eb5827e0e36b582d9bbfb8d7534f47188f09e7d3cce679fb1d12df347f')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1009, '1009#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- 'longpass@example.com' (password: 'QQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQ')
INSERT INTO accounts (id, email, country_code, battletag, first_name, last_name, birth_date)
VALUES (1010, 'longpass@example.com', 'DEU', 'Tester#1010', 'Legacy', 'User', '1986-08-17')
ON CONFLICT (email) DO UPDATE SET id = 1010;

INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
VALUES (1010,
  decode('0000000000000000000000000000000000000000000000000000000000000000','hex'),
  decode('888f85b4c0d2c7ef9f80c560320f2733f7482094c7a3e8b2ccf1ad8e38be6848c17c57239b5b8e7be33e92243ea5076d7fee0d2059c521cb13598879ed2cd63aa32822e52a3549d7c8f20b19709b3854e6271f4495ed2e983b640ee42909c57bad0932f13dd758f3751808278bc857001873a69c587a0119b9ce9095cde7eb584955aeaddc69af24b73e2f9b0abdc9c04fdf105f70fc35c6479a4b66d57da0716d67285098498b1fa722f6c3e62583ea62857168f16d5bcde645228358984ec5fa8466fef1ac71098e611630ced655203dcfa398a1523de217daa770ae0b824adfa3a01a1deb990cd801eec90e7187411e635025104d867164e77128b3731804','hex'),
  15000, 2,
  '96554480db1f8522811b3d025be53457ca5fdd57f4c6d6e19c8911cc60cee7f2')
ON CONFLICT (account_id) DO UPDATE SET
  srp_salt = EXCLUDED.srp_salt,
  srp_verifier = EXCLUDED.srp_verifier,
  srp_iterations = EXCLUDED.srp_iterations;

INSERT INTO game_accounts (account_id, name)
VALUES (1010, '1010#1')
ON CONFLICT (account_id, name) DO NOTHING;

-- Account licenses (required for GetLicenses / GetAccountState).
-- License IDs from the Blizzard product catalog (cascette-py catalog WoW).
-- 1106089 = WoW retail (current), 813186 = Dragonflight, 179712 = Shadowlands.
--
-- Scenarios:
--   1001 test          — full retail + active sub
--   1002 admin         — full retail + active sub
--   1003 player        — Dragonflight only + active sub
--   1004 minimal       — no licenses (should still log in)
--   1005 suspended     — Shadowlands + suspended
--   1006 expired       — Shadowlands + expired game time
--   1007 trial         — retail license + trial flag (no game time)
--   1008 banned         — Dragonflight + account banned
--   1009 classic_only   — no retail license, base game only
--   1010 legacy         — Shadowlands license, future renewal

-- Full retail: 1001 (test), 1002 (admin)
INSERT INTO account_licenses (account_id, license_id, level)
SELECT a.id, lic, 'exact'
FROM accounts a, (VALUES (1106089), (813186), (179712)) AS l(lic)
WHERE a.id IN (1001, 1002)
ON CONFLICT (account_id, license_id) WHERE game_account_id IS NULL DO NOTHING;

-- Dragonflight only: 1003 (player)
INSERT INTO account_licenses (account_id, license_id, level)
VALUES (1003, 813186, 'exact')
ON CONFLICT (account_id, license_id) WHERE game_account_id IS NULL DO NOTHING;

-- Shadowlands only: 1005 (suspended)
INSERT INTO account_licenses (account_id, license_id, level)
VALUES (1005, 179712, 'exact')
ON CONFLICT (account_id, license_id) WHERE game_account_id IS NULL DO NOTHING;

-- Shadowlands + future renewal: 1010 (legacy)
INSERT INTO account_licenses (account_id, license_id, level)
VALUES (1010, 179712, 'exact')
ON CONFLICT (account_id, license_id) WHERE game_account_id IS NULL DO NOTHING;

-- Retail + trial flag: 1007 (trial — no game time)
INSERT INTO account_licenses (account_id, license_id, level)
VALUES (1007, 1106089, 'trial')
ON CONFLICT (account_id, license_id) WHERE game_account_id IS NULL DO NOTHING;

-- Dragonflight + banned: 1008
INSERT INTO account_licenses (account_id, license_id, level)
VALUES (1008, 813186, 'exact')
ON CONFLICT (account_id, license_id) WHERE game_account_id IS NULL DO NOTHING;

-- Classic-only (base game, no retail license): 1009
-- No license rows — the base WoW game account grants Classic access.

-- Minimal (no licenses): 1004 — intentionally empty.

-- Game account suspension / game-time scenarios.
-- Active sub: game_time_expires 180 days from now.
UPDATE game_accounts SET game_time_expires = NOW() + interval '180 days'
WHERE account_id IN (1001, 1002, 1003, 1009);

-- Suspended: 1005. Suspension expires in 7 days.
UPDATE game_accounts SET is_suspended = TRUE, suspension_expires = NOW() + interval '7 days'
WHERE account_id = 1005;

-- Expired: 1006 (game time expired 1 day ago).
UPDATE game_accounts SET game_time_expires = NOW() - interval '1 day'
WHERE account_id = 1006;

-- Banned: 1008 (permanent).
UPDATE game_accounts SET is_banned = TRUE
WHERE account_id = 1008;

-- Trial: 1007 has license but no game_time_expires (zero time).

-- Account region assignments (1=US, 2=EU, 3=KR, 4=TW, 5=CN).
-- Account locale per country (mirrors locale::country_to_locale).
UPDATE accounts SET locale = 'enUS' WHERE id IN (1001, 1003, 1005, 1008);  -- USA
UPDATE accounts SET locale = 'deDE' WHERE id IN (1002, 1010);              -- DEU
UPDATE accounts SET locale = 'koKR' WHERE id = 1004;                        -- KOR
UPDATE accounts SET locale = 'enGB' WHERE id = 1006;                        -- GBR
UPDATE accounts SET locale = 'zhCN' WHERE id = 1007;                        -- CHN
UPDATE accounts SET locale = 'zhTW' WHERE id = 1009;                        -- TWN

UPDATE accounts SET region = 1 WHERE id = 1001;  -- test: US
UPDATE accounts SET region = 2 WHERE id = 1002;  -- admin: EU
UPDATE accounts SET region = 1 WHERE id = 1003;  -- player: US
UPDATE accounts SET region = 3 WHERE id = 1004;  -- minimal: KR
UPDATE accounts SET region = 1 WHERE id = 1005;  -- suspended: US
UPDATE accounts SET region = 2 WHERE id = 1006;  -- expired: EU
UPDATE accounts SET region = 5 WHERE id = 1007;  -- trial: CN
UPDATE accounts SET region = 1 WHERE id = 1008;  -- banned: US
UPDATE accounts SET region = 4 WHERE id = 1009;  -- classic-only: TW
UPDATE accounts SET region = 2 WHERE id = 1010;  -- legacy: EU

-- Authenticator (2FA) on admin@bnet.local for the 2FA login flow.
UPDATE accounts SET has_authenticator = TRUE WHERE id = 1002;
