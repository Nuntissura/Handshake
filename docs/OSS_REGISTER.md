# OSS REGISTER

**Authoritative Open Source Software Manifest**
**Status:** ACTIVE
**Updated:** 2026-01-01 (Added IntegrationMode column per SPEC v02.100 §11.7.4.4; added transitive deps from Cargo.lock)

> Scope: Captures all dependencies and dev/build tools declared in `Cargo.toml` (backend + Tauri) and `package.json` (frontend). Copyleft guard remains default-deny (GPL/AGPL only via `external_process`, none present today).

## Backend Direct – `src/backend/handshake_core/Cargo.toml`

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| axum | MIT | embedded_lib | Runtime | HTTP server (REST API) |
| serde | MIT/Apache-2.0 | embedded_lib | Runtime | Serialization/Deserialization |
| serde_json | MIT/Apache-2.0 | embedded_lib | Runtime | JSON handling |
| tokio | MIT | embedded_lib | Runtime | Async runtime (macros, process, time) |
| tower-http | MIT | embedded_lib | Runtime | HTTP middleware (CORS) |
| sqlx | MIT | embedded_lib | Runtime | DB driver (SQLite, migrations, chrono) |
| uuid | MIT/Apache-2.0 | embedded_lib | Runtime | UUID generation/serde |
| chrono | MIT/Apache-2.0 | embedded_lib | Runtime | Time handling (serde/clock) |
| tracing | MIT | embedded_lib | Runtime | Structured logging |
| tracing-subscriber | MIT | embedded_lib | Runtime | Logging sinks/filters (fmt/json) |
| tracing-appender | MIT | embedded_lib | Runtime | Log file appender |
| thiserror | MIT/Apache-2.0 | embedded_lib | Runtime | Error derivations |
| duckdb | MIT | embedded_lib | Runtime | Analytics / Flight Recorder (bundled) |
| reqwest | MIT/Apache-2.0 | embedded_lib | Runtime | HTTP client (Ollama integration) |
| async-trait | MIT/Apache-2.0 | embedded_lib | Runtime | Async trait support |
| once_cell | MIT/Apache-2.0 | embedded_lib | Runtime | Lazy init (metric sinks, registries) |
| sha2 | MIT/Apache-2.0 | embedded_lib | Runtime | SHA-256 hashing |
| hex | MIT/Apache-2.0 | embedded_lib | Runtime | Hex encoding |
| unicode-normalization | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode NFC normalization |
| tiktoken-rs | MIT | embedded_lib | Runtime | OpenAI tokenizer (optional) |
| tokenizers | Apache-2.0 | embedded_lib | Runtime | HuggingFace tokenizers (optional) |
| regex | MIT/Apache-2.0 | embedded_lib | Runtime | Regular expressions |
| zip | MIT | embedded_lib | Runtime | ZIP archive handling |
| tempfile | MIT/Apache-2.0 | embedded_lib | Dev | Temporary files for tests |

## Backend Transitive – `src/backend/handshake_core/Cargo.lock`

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| adler2 | MIT/Apache-2.0 | embedded_lib | Runtime | Adler-32 checksum |
| ahash | MIT/Apache-2.0 | embedded_lib | Runtime | Fast hashing |
| aho-corasick | MIT/Apache-2.0 | embedded_lib | Runtime | String matching |
| allocator-api2 | MIT/Apache-2.0 | embedded_lib | Runtime | Allocator API |
| android_system_properties | MIT/Apache-2.0 | embedded_lib | Runtime | Android props |
| anyhow | MIT/Apache-2.0 | embedded_lib | Runtime | Error handling |
| arbitrary | MIT/Apache-2.0 | embedded_lib | Runtime | Fuzzing support |
| arrayvec | MIT/Apache-2.0 | embedded_lib | Runtime | Stack arrays |
| arrow | Apache-2.0 | embedded_lib | Runtime | Arrow columnar format |
| arrow-arith | Apache-2.0 | embedded_lib | Runtime | Arrow arithmetic |
| arrow-array | Apache-2.0 | embedded_lib | Runtime | Arrow arrays |
| arrow-buffer | Apache-2.0 | embedded_lib | Runtime | Arrow buffers |
| arrow-cast | Apache-2.0 | embedded_lib | Runtime | Arrow casting |
| arrow-data | Apache-2.0 | embedded_lib | Runtime | Arrow data |
| arrow-ord | Apache-2.0 | embedded_lib | Runtime | Arrow ordering |
| arrow-row | Apache-2.0 | embedded_lib | Runtime | Arrow rows |
| arrow-schema | Apache-2.0 | embedded_lib | Runtime | Arrow schemas |
| arrow-select | Apache-2.0 | embedded_lib | Runtime | Arrow selection |
| arrow-string | Apache-2.0 | embedded_lib | Runtime | Arrow strings |
| atoi | MIT | embedded_lib | Runtime | String to int |
| atomic-waker | MIT/Apache-2.0 | embedded_lib | Runtime | Atomic waker |
| autocfg | MIT/Apache-2.0 | embedded_lib | Build | Auto-configuration |
| axum-core | MIT | embedded_lib | Runtime | Axum core types |
| base64 | MIT/Apache-2.0 | embedded_lib | Runtime | Base64 encoding |
| base64ct | MIT/Apache-2.0 | embedded_lib | Runtime | Constant-time base64 |
| bitflags | MIT/Apache-2.0 | embedded_lib | Runtime | Bit flags |
| bit-set | MIT/Apache-2.0 | embedded_lib | Runtime | Bit sets |
| bitvec | MIT | embedded_lib | Runtime | Bit vectors |
| bit-vec | MIT/Apache-2.0 | embedded_lib | Runtime | Bit vector |
| block-buffer | MIT/Apache-2.0 | embedded_lib | Runtime | Block buffer |
| borsh | MIT/Apache-2.0 | embedded_lib | Runtime | Binary serialization |
| borsh-derive | MIT/Apache-2.0 | embedded_lib | Runtime | Borsh derive |
| bstr | MIT/Apache-2.0 | embedded_lib | Runtime | Byte strings |
| bumpalo | MIT/Apache-2.0 | embedded_lib | Runtime | Bump allocator |
| bytecheck | MIT | embedded_lib | Runtime | Byte validation |
| bytecheck_derive | MIT | embedded_lib | Runtime | Bytecheck derive |
| byteorder | MIT/Unlicense | embedded_lib | Runtime | Byte order |
| bytes | MIT | embedded_lib | Runtime | Byte buffers |
| cast | MIT/Apache-2.0 | embedded_lib | Runtime | Numeric casting |
| cc | MIT/Apache-2.0 | embedded_lib | Build | C compiler |
| cfg_aliases | MIT | embedded_lib | Build | Config aliases |
| cfg-if | MIT/Apache-2.0 | embedded_lib | Runtime | Conditional compilation |
| comfy-table | MIT | embedded_lib | Runtime | Table formatting |
| concurrent-queue | MIT/Apache-2.0 | embedded_lib | Runtime | Concurrent queue |
| console | MIT | embedded_lib | Runtime | Terminal console |
| const-oid | MIT/Apache-2.0 | embedded_lib | Runtime | Const OID |
| const-random | MIT/Apache-2.0 | embedded_lib | Runtime | Const random |
| const-random-macro | MIT/Apache-2.0 | embedded_lib | Runtime | Const random macro |
| core-foundation | MIT/Apache-2.0 | embedded_lib | Runtime | macOS core foundation |
| core-foundation-sys | MIT/Apache-2.0 | embedded_lib | Runtime | Core foundation sys |
| cpufeatures | MIT/Apache-2.0 | embedded_lib | Runtime | CPU features |
| crc | MIT/Apache-2.0 | embedded_lib | Runtime | CRC checksums |
| crc32fast | MIT/Apache-2.0 | embedded_lib | Runtime | Fast CRC32 |
| crc-catalog | MIT/Apache-2.0 | embedded_lib | Runtime | CRC catalog |
| crossbeam-channel | MIT/Apache-2.0 | embedded_lib | Runtime | Crossbeam channels |
| crossbeam-deque | MIT/Apache-2.0 | embedded_lib | Runtime | Work-stealing deque |
| crossbeam-epoch | MIT/Apache-2.0 | embedded_lib | Runtime | Epoch GC |
| crossbeam-queue | MIT/Apache-2.0 | embedded_lib | Runtime | Concurrent queues |
| crossbeam-utils | MIT/Apache-2.0 | embedded_lib | Runtime | Crossbeam utilities |
| crunchy | MIT | embedded_lib | Runtime | Loop unrolling |
| crypto-common | MIT/Apache-2.0 | embedded_lib | Runtime | Crypto common |
| darling | MIT | embedded_lib | Runtime | Derive helpers |
| darling_core | MIT | embedded_lib | Runtime | Darling core |
| darling_macro | MIT | embedded_lib | Runtime | Darling macros |
| der | MIT/Apache-2.0 | embedded_lib | Runtime | DER encoding |
| deranged | MIT/Apache-2.0 | embedded_lib | Runtime | Ranged integers |
| derive_arbitrary | MIT/Apache-2.0 | embedded_lib | Runtime | Arbitrary derive |
| derive_builder | MIT/Apache-2.0 | embedded_lib | Runtime | Builder derive |
| derive_builder_core | MIT/Apache-2.0 | embedded_lib | Runtime | Builder core |
| derive_builder_macro | MIT/Apache-2.0 | embedded_lib | Runtime | Builder macro |
| digest | MIT/Apache-2.0 | embedded_lib | Runtime | Digest traits |
| displaydoc | MIT/Apache-2.0 | embedded_lib | Runtime | Display docs |
| dotenvy | MIT | embedded_lib | Runtime | Dotenv loading |
| either | MIT/Apache-2.0 | embedded_lib | Runtime | Either type |
| encode_unicode | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode encoding |
| encoding_rs | MIT/Apache-2.0 | embedded_lib | Runtime | Character encoding |
| equivalent | MIT/Apache-2.0 | embedded_lib | Runtime | Equivalence trait |
| errno | MIT/Apache-2.0 | embedded_lib | Runtime | Errno handling |
| esaxx-rs | MIT | embedded_lib | Runtime | ESA-XX suffix array |
| etcetera | MIT/Apache-2.0 | embedded_lib | Runtime | XDG directories |
| event-listener | MIT/Apache-2.0 | embedded_lib | Runtime | Event listener |
| fallible-iterator | MIT/Apache-2.0 | embedded_lib | Runtime | Fallible iterators |
| fallible-streaming-iterator | MIT/Apache-2.0 | embedded_lib | Runtime | Streaming iterators |
| fancy-regex | MIT | embedded_lib | Runtime | Advanced regex |
| fastrand | MIT/Apache-2.0 | embedded_lib | Runtime | Fast random |
| filetime | MIT/Apache-2.0 | embedded_lib | Runtime | File times |
| find-msvc-tools | MIT | embedded_lib | Build | MSVC tools |
| flate2 | MIT/Apache-2.0 | embedded_lib | Runtime | Deflate compression |
| flume | MIT/Apache-2.0 | embedded_lib | Runtime | Channel library |
| fnv | MIT/Apache-2.0 | embedded_lib | Runtime | FNV hashing |
| foldhash | MIT/Apache-2.0 | embedded_lib | Runtime | Folded hashing |
| foreign-types | MIT/Apache-2.0 | embedded_lib | Runtime | Foreign types |
| foreign-types-shared | MIT/Apache-2.0 | embedded_lib | Runtime | Foreign types shared |
| form_urlencoded | MIT/Apache-2.0 | embedded_lib | Runtime | URL encoding |
| funty | MIT | embedded_lib | Runtime | Fundamental types |
| futures-channel | MIT/Apache-2.0 | embedded_lib | Runtime | Futures channel |
| futures-core | MIT/Apache-2.0 | embedded_lib | Runtime | Futures core |
| futures-executor | MIT/Apache-2.0 | embedded_lib | Runtime | Futures executor |
| futures-intrusive | MIT/Apache-2.0 | embedded_lib | Runtime | Intrusive futures |
| futures-io | MIT/Apache-2.0 | embedded_lib | Runtime | Futures I/O |
| futures-sink | MIT/Apache-2.0 | embedded_lib | Runtime | Futures sink |
| futures-task | MIT/Apache-2.0 | embedded_lib | Runtime | Futures task |
| futures-util | MIT/Apache-2.0 | embedded_lib | Runtime | Futures utilities |
| generic-array | MIT | embedded_lib | Runtime | Generic arrays |
| getrandom | MIT/Apache-2.0 | embedded_lib | Runtime | Random bytes |
| h2 | MIT | embedded_lib | Runtime | HTTP/2 |
| half | MIT/Apache-2.0 | embedded_lib | Runtime | Half-precision floats |
| handshake_core | MIT | embedded_lib | Runtime | This project |
| hashbrown | MIT/Apache-2.0 | embedded_lib | Runtime | Hash tables |
| hashlink | MIT/Apache-2.0 | embedded_lib | Runtime | Linked hash map |
| heck | MIT/Apache-2.0 | embedded_lib | Runtime | Case conversion |
| hkdf | MIT/Apache-2.0 | embedded_lib | Runtime | HKDF |
| hmac | MIT/Apache-2.0 | embedded_lib | Runtime | HMAC |
| home | MIT/Apache-2.0 | embedded_lib | Runtime | Home directory |
| http | MIT/Apache-2.0 | embedded_lib | Runtime | HTTP types |
| httparse | MIT/Apache-2.0 | embedded_lib | Runtime | HTTP parsing |
| http-body | MIT | embedded_lib | Runtime | HTTP body |
| http-body-util | MIT | embedded_lib | Runtime | HTTP body utils |
| httpdate | MIT/Apache-2.0 | embedded_lib | Runtime | HTTP date |
| hyper | MIT | embedded_lib | Runtime | HTTP client/server |
| hyper-rustls | MIT/Apache-2.0 | embedded_lib | Runtime | Hyper TLS |
| hyper-tls | MIT/Apache-2.0 | embedded_lib | Runtime | Hyper TLS native |
| hyper-util | MIT | embedded_lib | Runtime | Hyper utilities |
| iana-time-zone | MIT/Apache-2.0 | embedded_lib | Runtime | IANA time zones |
| iana-time-zone-haiku | MIT/Apache-2.0 | embedded_lib | Runtime | Haiku time zones |
| icu_collections | Unicode-3.0 | embedded_lib | Runtime | ICU collections |
| icu_locale_core | Unicode-3.0 | embedded_lib | Runtime | ICU locale |
| icu_normalizer | Unicode-3.0 | embedded_lib | Runtime | ICU normalizer |
| icu_normalizer_data | Unicode-3.0 | embedded_lib | Runtime | ICU normalizer data |
| icu_properties | Unicode-3.0 | embedded_lib | Runtime | ICU properties |
| icu_properties_data | Unicode-3.0 | embedded_lib | Runtime | ICU properties data |
| icu_provider | Unicode-3.0 | embedded_lib | Runtime | ICU provider |
| ident_case | MIT/Apache-2.0 | embedded_lib | Runtime | Identifier case |
| idna | MIT/Apache-2.0 | embedded_lib | Runtime | IDNA |
| idna_adapter | MIT/Apache-2.0 | embedded_lib | Runtime | IDNA adapter |
| indexmap | MIT/Apache-2.0 | embedded_lib | Runtime | Indexed map |
| indicatif | MIT | embedded_lib | Runtime | Progress bars |
| ipnet | MIT/Apache-2.0 | embedded_lib | Runtime | IP networks |
| iri-string | MIT/Apache-2.0 | embedded_lib | Runtime | IRI strings |
| itertools | MIT/Apache-2.0 | embedded_lib | Runtime | Iterator tools |
| itoa | MIT/Apache-2.0 | embedded_lib | Runtime | Integer to string |
| jobserver | MIT/Apache-2.0 | embedded_lib | Build | Job server |
| js-sys | MIT/Apache-2.0 | embedded_lib | Runtime | JS bindings |
| lazy_static | MIT/Apache-2.0 | embedded_lib | Runtime | Lazy statics |
| lexical-core | MIT/Apache-2.0 | embedded_lib | Runtime | Lexical core |
| lexical-parse-float | MIT/Apache-2.0 | embedded_lib | Runtime | Float parsing |
| lexical-parse-integer | MIT/Apache-2.0 | embedded_lib | Runtime | Integer parsing |
| lexical-util | MIT/Apache-2.0 | embedded_lib | Runtime | Lexical utilities |
| lexical-write-float | MIT/Apache-2.0 | embedded_lib | Runtime | Float writing |
| lexical-write-integer | MIT/Apache-2.0 | embedded_lib | Runtime | Integer writing |
| libc | MIT/Apache-2.0 | embedded_lib | Runtime | C library bindings |
| libduckdb-sys | MIT | embedded_lib | Runtime | DuckDB FFI |
| libm | MIT/Apache-2.0 | embedded_lib | Runtime | Math library |
| libredox | MIT | embedded_lib | Runtime | Redox syscalls |
| libsqlite3-sys | MIT | embedded_lib | Runtime | SQLite FFI |
| libz-rs-sys | Zlib | embedded_lib | Runtime | Zlib bindings |
| linux-raw-sys | MIT/Apache-2.0 | embedded_lib | Runtime | Linux syscalls |
| litemap | Unicode-3.0 | embedded_lib | Runtime | Literal map |
| lock_api | MIT/Apache-2.0 | embedded_lib | Runtime | Lock API |
| log | MIT/Apache-2.0 | embedded_lib | Runtime | Logging facade |
| lru-slab | MIT | embedded_lib | Runtime | LRU slab |
| macro_rules_attribute | MIT | embedded_lib | Runtime | Macro rules |
| macro_rules_attribute-proc_macro | MIT | embedded_lib | Runtime | Macro rules proc |
| matchers | MIT | embedded_lib | Runtime | Pattern matchers |
| matchit | MIT/Apache-2.0 | embedded_lib | Runtime | URL matching |
| md-5 | MIT/Apache-2.0 | embedded_lib | Runtime | MD5 hashing |
| memchr | MIT/Unlicense | embedded_lib | Runtime | Byte search |
| mime | MIT/Apache-2.0 | embedded_lib | Runtime | MIME types |
| minimal-lexical | MIT/Apache-2.0 | embedded_lib | Runtime | Minimal lexical |
| miniz_oxide | MIT/Zlib/Apache-2.0 | embedded_lib | Runtime | Miniz oxide |
| mio | MIT | embedded_lib | Runtime | I/O library |
| monostate | MIT/Apache-2.0 | embedded_lib | Runtime | Monostate |
| monostate-impl | MIT/Apache-2.0 | embedded_lib | Runtime | Monostate impl |
| native-tls | MIT/Apache-2.0 | embedded_lib | Runtime | Native TLS |
| nom | MIT | embedded_lib | Runtime | Parser combinators |
| nu-ansi-term | MIT | embedded_lib | Runtime | ANSI terminal |
| num | MIT/Apache-2.0 | embedded_lib | Runtime | Numeric types |
| number_prefix | MIT | embedded_lib | Runtime | Number prefix |
| num-bigint | MIT/Apache-2.0 | embedded_lib | Runtime | Big integers |
| num-bigint-dig | MIT/Apache-2.0 | embedded_lib | Runtime | Big integers dig |
| num-complex | MIT/Apache-2.0 | embedded_lib | Runtime | Complex numbers |
| num-conv | MIT/Apache-2.0 | embedded_lib | Runtime | Numeric conversion |
| num-integer | MIT/Apache-2.0 | embedded_lib | Runtime | Integer traits |
| num-iter | MIT/Apache-2.0 | embedded_lib | Runtime | Numeric iterators |
| num-rational | MIT/Apache-2.0 | embedded_lib | Runtime | Rational numbers |
| num-traits | MIT/Apache-2.0 | embedded_lib | Runtime | Numeric traits |
| onig | MIT | embedded_lib | Runtime | Oniguruma regex |
| onig_sys | MIT | embedded_lib | Runtime | Oniguruma FFI |
| openssl | Apache-2.0 | embedded_lib | Runtime | OpenSSL bindings |
| openssl-macros | MIT/Apache-2.0 | embedded_lib | Runtime | OpenSSL macros |
| openssl-probe | MIT/Apache-2.0 | embedded_lib | Runtime | OpenSSL probe |
| openssl-sys | MIT | embedded_lib | Runtime | OpenSSL FFI |
| parking | MIT/Apache-2.0 | embedded_lib | Runtime | Thread parking |
| parking_lot | MIT/Apache-2.0 | embedded_lib | Runtime | Parking lot |
| parking_lot_core | MIT/Apache-2.0 | embedded_lib | Runtime | Parking lot core |
| paste | MIT/Apache-2.0 | embedded_lib | Runtime | Macro pasting |
| pem-rfc7468 | MIT/Apache-2.0 | embedded_lib | Runtime | PEM encoding |
| percent-encoding | MIT/Apache-2.0 | embedded_lib | Runtime | Percent encoding |
| pin-project-lite | MIT/Apache-2.0 | embedded_lib | Runtime | Pin project |
| pin-utils | MIT/Apache-2.0 | embedded_lib | Runtime | Pin utilities |
| pkcs1 | MIT/Apache-2.0 | embedded_lib | Runtime | PKCS#1 |
| pkcs8 | MIT/Apache-2.0 | embedded_lib | Runtime | PKCS#8 |
| pkg-config | MIT/Apache-2.0 | embedded_lib | Build | Pkg-config |
| portable-atomic | MIT/Apache-2.0 | embedded_lib | Runtime | Portable atomics |
| potential_utf | MIT/Apache-2.0 | embedded_lib | Runtime | Potential UTF |
| powerfmt | MIT/Apache-2.0 | embedded_lib | Runtime | Power formatting |
| ppv-lite86 | MIT/Apache-2.0 | embedded_lib | Runtime | SIMD vectors |
| proc-macro2 | MIT/Apache-2.0 | embedded_lib | Build | Proc macros |
| proc-macro-crate | MIT/Apache-2.0 | embedded_lib | Build | Proc macro crate |
| ptr_meta | MIT | embedded_lib | Runtime | Pointer metadata |
| ptr_meta_derive | MIT | embedded_lib | Runtime | Pointer meta derive |
| quinn | MIT/Apache-2.0 | embedded_lib | Runtime | QUIC |
| quinn-proto | MIT/Apache-2.0 | embedded_lib | Runtime | QUIC protocol |
| quinn-udp | MIT/Apache-2.0 | embedded_lib | Runtime | QUIC UDP |
| quote | MIT/Apache-2.0 | embedded_lib | Build | Quasi-quoting |
| radium | MIT | embedded_lib | Runtime | Radium |
| rand | MIT/Apache-2.0 | embedded_lib | Runtime | Random numbers |
| rand_chacha | MIT/Apache-2.0 | embedded_lib | Runtime | ChaCha RNG |
| rand_core | MIT/Apache-2.0 | embedded_lib | Runtime | RNG core |
| rayon | MIT/Apache-2.0 | embedded_lib | Runtime | Parallel iteration |
| rayon-cond | MIT/Apache-2.0 | embedded_lib | Runtime | Rayon conditional |
| rayon-core | MIT/Apache-2.0 | embedded_lib | Runtime | Rayon core |
| redox_syscall | MIT | embedded_lib | Runtime | Redox syscalls |
| r-efi | MIT/Apache-2.0 | embedded_lib | Runtime | UEFI types |
| regex-automata | MIT/Apache-2.0 | embedded_lib | Runtime | Regex automata |
| regex-syntax | MIT/Apache-2.0 | embedded_lib | Runtime | Regex syntax |
| rend | MIT | embedded_lib | Runtime | Rend |
| ring | MIT | embedded_lib | Runtime | Crypto primitives |
| rkyv | MIT | embedded_lib | Runtime | Zero-copy serialization |
| rkyv_derive | MIT | embedded_lib | Runtime | Rkyv derive |
| rsa | MIT/Apache-2.0 | embedded_lib | Runtime | RSA crypto |
| rust_decimal | MIT | embedded_lib | Runtime | Decimal type |
| rustc-hash | MIT/Apache-2.0 | embedded_lib | Runtime | Rustc hash |
| rustix | MIT/Apache-2.0 | embedded_lib | Runtime | Unix APIs |
| rustls | MIT/Apache-2.0 | embedded_lib | Runtime | TLS library |
| rustls-pki-types | MIT/Apache-2.0 | embedded_lib | Runtime | PKI types |
| rustls-webpki | ISC | embedded_lib | Runtime | WebPKI |
| rustversion | MIT/Apache-2.0 | embedded_lib | Build | Rust version |
| ryu | Apache-2.0/BSL-1.0 | embedded_lib | Runtime | Float to string |
| schannel | MIT | embedded_lib | Runtime | Windows TLS |
| scopeguard | MIT/Apache-2.0 | embedded_lib | Runtime | Scope guard |
| seahash | MIT | embedded_lib | Runtime | SeaHash |
| security-framework | MIT/Apache-2.0 | embedded_lib | Runtime | macOS security |
| security-framework-sys | MIT/Apache-2.0 | embedded_lib | Runtime | Security framework sys |
| serde_core | MIT/Apache-2.0 | embedded_lib | Runtime | Serde core |
| serde_derive | MIT/Apache-2.0 | embedded_lib | Runtime | Serde derive |
| serde_path_to_error | MIT/Apache-2.0 | embedded_lib | Runtime | Serde path error |
| serde_urlencoded | MIT/Apache-2.0 | embedded_lib | Runtime | URL encoding |
| sha1 | MIT/Apache-2.0 | embedded_lib | Runtime | SHA-1 |
| sharded-slab | MIT | embedded_lib | Runtime | Sharded slab |
| shlex | MIT/Apache-2.0 | embedded_lib | Runtime | Shell lexer |
| signal-hook-registry | MIT/Apache-2.0 | embedded_lib | Runtime | Signal registry |
| signature | MIT/Apache-2.0 | embedded_lib | Runtime | Crypto signatures |
| simd-adler32 | MIT | embedded_lib | Runtime | SIMD Adler32 |
| simdutf8 | MIT/Apache-2.0 | embedded_lib | Runtime | SIMD UTF-8 |
| slab | MIT | embedded_lib | Runtime | Slab allocator |
| smallvec | MIT/Apache-2.0 | embedded_lib | Runtime | Small vectors |
| socket2 | MIT/Apache-2.0 | embedded_lib | Runtime | Socket utilities |
| spin | MIT | embedded_lib | Runtime | Spinlocks |
| spki | MIT/Apache-2.0 | embedded_lib | Runtime | SPKI |
| spm_precompiled | MIT | embedded_lib | Runtime | SPM precompiled |
| sqlx-core | MIT | embedded_lib | Runtime | SQLx core |
| sqlx-macros | MIT | embedded_lib | Runtime | SQLx macros |
| sqlx-macros-core | MIT | embedded_lib | Runtime | SQLx macros core |
| sqlx-mysql | MIT | embedded_lib | Runtime | SQLx MySQL |
| sqlx-postgres | MIT | embedded_lib | Runtime | SQLx Postgres |
| sqlx-sqlite | MIT | embedded_lib | Runtime | SQLx SQLite |
| stable_deref_trait | MIT/Apache-2.0 | embedded_lib | Runtime | Stable deref |
| stringprep | MIT/Apache-2.0 | embedded_lib | Runtime | Stringprep |
| strsim | MIT | embedded_lib | Runtime | String similarity |
| strum | MIT | embedded_lib | Runtime | Enum utilities |
| strum_macros | MIT | embedded_lib | Runtime | Strum macros |
| subtle | BSD-3-Clause | embedded_lib | Runtime | Constant-time ops |
| syn | MIT/Apache-2.0 | embedded_lib | Build | Syntax parsing |
| sync_wrapper | Apache-2.0 | embedded_lib | Runtime | Sync wrapper |
| synstructure | MIT | embedded_lib | Runtime | Syn structure |
| system-configuration | MIT/Apache-2.0 | embedded_lib | Runtime | macOS config |
| system-configuration-sys | MIT/Apache-2.0 | embedded_lib | Runtime | System config sys |
| tap | MIT | embedded_lib | Runtime | Tap trait |
| tar | MIT/Apache-2.0 | embedded_lib | Runtime | Tar archives |
| thiserror-impl | MIT/Apache-2.0 | embedded_lib | Runtime | Thiserror impl |
| thread_local | MIT/Apache-2.0 | embedded_lib | Runtime | Thread local |
| time | MIT/Apache-2.0 | embedded_lib | Runtime | Time library |
| time-core | MIT/Apache-2.0 | embedded_lib | Runtime | Time core |
| time-macros | MIT/Apache-2.0 | embedded_lib | Runtime | Time macros |
| tiny-keccak | CC0-1.0 | embedded_lib | Runtime | Keccak hashing |
| tinystr | Unicode-3.0 | embedded_lib | Runtime | Tiny strings |
| tinyvec | MIT/Apache-2.0/Zlib | embedded_lib | Runtime | Tiny vectors |
| tinyvec_macros | MIT/Apache-2.0/Zlib | embedded_lib | Runtime | Tinyvec macros |
| tokio-macros | MIT | embedded_lib | Runtime | Tokio macros |
| tokio-native-tls | MIT | embedded_lib | Runtime | Tokio native TLS |
| tokio-rustls | MIT/Apache-2.0 | embedded_lib | Runtime | Tokio rustls |
| tokio-stream | MIT | embedded_lib | Runtime | Tokio streams |
| tokio-util | MIT | embedded_lib | Runtime | Tokio utilities |
| toml_datetime | MIT/Apache-2.0 | embedded_lib | Runtime | TOML datetime |
| toml_edit | MIT/Apache-2.0 | embedded_lib | Runtime | TOML editing |
| toml_parser | MIT/Apache-2.0 | embedded_lib | Runtime | TOML parser |
| tower | MIT | embedded_lib | Runtime | Service abstraction |
| tower-layer | MIT | embedded_lib | Runtime | Tower layers |
| tower-service | MIT | embedded_lib | Runtime | Tower service |
| tracing-attributes | MIT | embedded_lib | Runtime | Tracing attributes |
| tracing-core | MIT | embedded_lib | Runtime | Tracing core |
| tracing-log | MIT | embedded_lib | Runtime | Tracing log |
| tracing-serde | MIT | embedded_lib | Runtime | Tracing serde |
| try-lock | MIT | embedded_lib | Runtime | Try lock |
| typenum | MIT/Apache-2.0 | embedded_lib | Runtime | Type-level numbers |
| unicode_categories | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode categories |
| unicode-bidi | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode bidi |
| unicode-ident | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode identifiers |
| unicode-normalization-alignments | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode alignments |
| unicode-properties | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode properties |
| unicode-segmentation | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode segmentation |
| unicode-width | MIT/Apache-2.0 | embedded_lib | Runtime | Unicode width |
| untrusted | ISC | embedded_lib | Runtime | Untrusted input |
| url | MIT/Apache-2.0 | embedded_lib | Runtime | URL parsing |
| utf8_iter | MIT/Apache-2.0 | embedded_lib | Runtime | UTF-8 iterator |
| valuable | MIT | embedded_lib | Runtime | Valuable trait |
| vcpkg | MIT/Apache-2.0 | embedded_lib | Build | Vcpkg integration |
| version_check | MIT/Apache-2.0 | embedded_lib | Build | Version checking |
| want | MIT | embedded_lib | Runtime | Want signaling |
| wasi | MIT/Apache-2.0 | embedded_lib | Runtime | WASI bindings |
| wasip2 | MIT/Apache-2.0 | embedded_lib | Runtime | WASI preview 2 |
| wasite | MIT | embedded_lib | Runtime | Wasite |
| wasm-bindgen | MIT/Apache-2.0 | embedded_lib | Runtime | WASM bindings |
| wasm-bindgen-futures | MIT/Apache-2.0 | embedded_lib | Runtime | WASM futures |
| wasm-bindgen-macro | MIT/Apache-2.0 | embedded_lib | Runtime | WASM macro |
| wasm-bindgen-macro-support | MIT/Apache-2.0 | embedded_lib | Runtime | WASM macro support |
| wasm-bindgen-shared | MIT/Apache-2.0 | embedded_lib | Runtime | WASM shared |
| webpki-roots | MPL-2.0 | embedded_lib | Runtime | WebPKI roots |
| web-sys | MIT/Apache-2.0 | embedded_lib | Runtime | Web APIs |
| web-time | MIT/Apache-2.0 | embedded_lib | Runtime | Web time |
| whoami | MIT/Apache-2.0 | embedded_lib | Runtime | User info |
| windows_aarch64_gnullvm | MIT/Apache-2.0 | embedded_lib | Runtime | Windows ARM64 |
| windows_aarch64_msvc | MIT/Apache-2.0 | embedded_lib | Runtime | Windows ARM64 MSVC |
| windows_i686_gnu | MIT/Apache-2.0 | embedded_lib | Runtime | Windows i686 GNU |
| windows_i686_gnullvm | MIT/Apache-2.0 | embedded_lib | Runtime | Windows i686 GNULLVM |
| windows_i686_msvc | MIT/Apache-2.0 | embedded_lib | Runtime | Windows i686 MSVC |
| windows_x86_64_gnu | MIT/Apache-2.0 | embedded_lib | Runtime | Windows x64 GNU |
| windows_x86_64_gnullvm | MIT/Apache-2.0 | embedded_lib | Runtime | Windows x64 GNULLVM |
| windows_x86_64_msvc | MIT/Apache-2.0 | embedded_lib | Runtime | Windows x64 MSVC |
| windows-core | MIT/Apache-2.0 | embedded_lib | Runtime | Windows core |
| windows-implement | MIT/Apache-2.0 | embedded_lib | Runtime | Windows implement |
| windows-interface | MIT/Apache-2.0 | embedded_lib | Runtime | Windows interface |
| windows-link | MIT/Apache-2.0 | embedded_lib | Runtime | Windows link |
| windows-registry | MIT/Apache-2.0 | embedded_lib | Runtime | Windows registry |
| windows-result | MIT/Apache-2.0 | embedded_lib | Runtime | Windows result |
| windows-strings | MIT/Apache-2.0 | embedded_lib | Runtime | Windows strings |
| windows-sys | MIT/Apache-2.0 | embedded_lib | Runtime | Windows sys |
| windows-targets | MIT/Apache-2.0 | embedded_lib | Runtime | Windows targets |
| winnow | MIT | embedded_lib | Runtime | Parser library |
| wit-bindgen | MIT/Apache-2.0 | embedded_lib | Runtime | WIT bindings |
| writeable | Unicode-3.0 | embedded_lib | Runtime | Writeable trait |
| wyz | MIT | embedded_lib | Runtime | Utility macros |
| xattr | MIT/Apache-2.0 | embedded_lib | Runtime | Extended attrs |
| yoke | Unicode-3.0 | embedded_lib | Runtime | Yoke borrowing |
| yoke-derive | Unicode-3.0 | embedded_lib | Runtime | Yoke derive |
| zerocopy | BSD-2-Clause | embedded_lib | Runtime | Zero-copy parsing |
| zerocopy-derive | BSD-2-Clause | embedded_lib | Runtime | Zerocopy derive |
| zerofrom | Unicode-3.0 | embedded_lib | Runtime | Zerofrom |
| zerofrom-derive | Unicode-3.0 | embedded_lib | Runtime | Zerofrom derive |
| zeroize | MIT/Apache-2.0 | embedded_lib | Runtime | Memory zeroing |
| zerotrie | Unicode-3.0 | embedded_lib | Runtime | Zero-copy trie |
| zerovec | Unicode-3.0 | embedded_lib | Runtime | Zero-copy vectors |
| zerovec-derive | Unicode-3.0 | embedded_lib | Runtime | Zerovec derive |
| zlib-rs | Zlib | embedded_lib | Runtime | Zlib pure Rust |
| zopfli | Apache-2.0 | embedded_lib | Runtime | Zopfli compression |

## Desktop Shell – `app/src-tauri/Cargo.toml`

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| tauri | MIT/Apache-2.0 | embedded_lib | Runtime | Desktop shell / IPC bridge |
| tauri-plugin-opener | MIT/Apache-2.0 | embedded_lib | Runtime | Safe "open" integration |
| tauri-build | MIT/Apache-2.0 | embedded_lib | Build | Tauri build script support |

## Frontend Runtime – `app/package.json` dependencies

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| @excalidraw/excalidraw | MIT | embedded_lib | Runtime | Canvas / whiteboard |
| @tauri-apps/api | MIT/Apache-2.0 | embedded_lib | Runtime | Tauri IPC and shell APIs |
| @tauri-apps/plugin-opener | MIT/Apache-2.0 | embedded_lib | Runtime | Link/file opener bridge |
| @tiptap/core | MIT | embedded_lib | Runtime | Rich-text core |
| @tiptap/extension-collaboration | MIT | embedded_lib | Runtime | CRDT-backed editing |
| @tiptap/react | MIT | embedded_lib | Runtime | React bindings for TipTap |
| @tiptap/starter-kit | MIT | embedded_lib | Runtime | Default editor nodes/marks |
| react | MIT | embedded_lib | Runtime | UI framework |
| react-dom | MIT | embedded_lib | Runtime | React DOM renderer |
| yjs | MIT | embedded_lib | Runtime | CRDT collaboration |

## Frontend Tooling & Tests – `app/package.json` devDependencies

| Component | License | IntegrationMode | Scope | Purpose |
| --- | --- | --- | --- | --- |
| @eslint/js | MIT | embedded_lib | Dev | ESLint config |
| @tauri-apps/cli | MIT/Apache-2.0 | embedded_lib | Dev | Tauri CLI/build |
| @testing-library/jest-dom | MIT | embedded_lib | Test | Jest DOM matchers |
| @testing-library/react | MIT | embedded_lib | Test | React testing utilities |
| @types/jsdom | MIT | embedded_lib | Dev | TypeScript types |
| @types/react | MIT | embedded_lib | Dev | TypeScript types |
| @types/react-dom | MIT | embedded_lib | Dev | TypeScript types |
| @typescript-eslint/eslint-plugin | MIT | embedded_lib | Dev | ESLint rules for TS |
| @typescript-eslint/parser | MIT | embedded_lib | Dev | ESLint parser for TS |
| @vitejs/plugin-react | MIT | embedded_lib | Dev | Vite React plugin |
| dependency-cruiser | MIT | embedded_lib | Dev | Dependency graph linting |
| eslint | MIT | embedded_lib | Dev | Lint runner |
| eslint-plugin-react-hooks | MIT | embedded_lib | Dev | Hooks lint rules |
| globals | MIT | embedded_lib | Dev | Global definitions for ESLint |
| jsdom | MIT | embedded_lib | Test | DOM emulation for tests |
| typescript | Apache-2.0 | embedded_lib | Dev | TypeScript compiler |
| vite | MIT | embedded_lib | Dev | Bundler |
| vitest | MIT | embedded_lib | Test | Test runner |

## Governance & Enforcement

1) **Copyleft isolation (§11.10.4.2):** GPL/AGPL components MUST use `external_process` integration mode (not `embedded_lib` or `external_service`). None present in current manifests.
2) **Coverage rule:** Every dependency declared in `Cargo.lock`/`package.json` must appear in this register; update this file whenever dependencies change.
3) **Security gate:** Supply-chain checks (e.g., `just validate`, `cargo deny`, npm audit equivalents) must be remediated within 48 hours or blocked at merge.
4) **Evidence:** Register updates should cite the manifest/lock source in PR descriptions to keep provenance auditable.
5) **Enforcement test:** `cargo test oss_register_enforcement` validates coverage and copyleft isolation per §11.10.4.2.
