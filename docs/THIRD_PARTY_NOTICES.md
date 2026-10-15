# DCinVox — Third-Party Notices / 사용한 공개 소프트웨어

[한국어·English](THIRD_PARTY_NOTICES.md) · [日本語](THIRD_PARTY_NOTICES.ja.md)

DCinVox © HolicJin. DCinVox includes the following third-party software and models. 아래 공개 소프트웨어와 AI 모델을 포함합니다.

## 함께 배포되는 프로그램·모델 / Bundled components

| 구성요소 | 용도 | 라이선스 | 출처 |
|---|---|---|---|
| FFmpeg 9.0.2 (LGPL 빌드, 별도 실행파일) | 영상·소리 읽기/쓰기 | LGPL-2.1-or-later | https://ffmpeg.org — 소스와 빌드 스크립트(`tools/build_ffmpeg_macos.sh`)는 요청 시 제공 |
| whisper.cpp / ggml | 음성 인식 엔진 | MIT | https://github.com/ggml-org/whisper.cpp |
| Whisper large-v3 (ggml q5_0 변환본) | 음성 인식 모델 | MIT (OpenAI) | https://huggingface.co/ggerganov/whisper.cpp |
| wav2vec2-xlsr-53-espeak-cv-ft (ONNX int8 변환본) | 음소 정렬 모델 | Apache-2.0 (Meta AI) | https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft |
| ONNX Runtime | 정렬 모델 실행 | MIT (Microsoft) | https://github.com/microsoft/onnxruntime |
| WORLD vocoder | 음높이 변환 | Modified BSD (M. Morise) | https://github.com/mmorise/World |
| SQLite | 색인 저장 | Public domain | https://sqlite.org |
| Tauri | 앱 틀 | MIT / Apache-2.0 | https://tauri.app |

### FFmpeg (LGPL) 안내

DCinVox는 FFmpeg를 별도 실행파일(`ffmpeg`)로 포함하며 GPL·nonfree 구성요소 없이 빌드했습니다. 사용자는 이 실행파일을 직접 빌드한 FFmpeg로 바꿀 수 있습니다. 해당 버전의 소스 코드와 빌드 설정은 요청하시면 제공합니다.
DCinVox ships FFmpeg as a separate executable built without GPL or nonfree components. You may replace it with your own build. The corresponding source code and build configuration are available on request.

## Rust 라이브러리 / Rust crates (452)

### MIT OR Apache-2.0 (215)

android_system_properties 0.1.6, anyhow 1.0.104, base64 0.21.7, base64 0.22.1, base64 0.23.1, bitflags 2.13.2, block-buffer 0.10.4, bumpalo 3.20.3, camino 1.2.6, cargo-platform 0.1.9, cfg-if 1.0.5, chrono 0.4.45, cookie 0.18.2, core-foundation 0.10.1, core-foundation-sys 0.8.7, core-graphics 0.25.0, core-graphics-types 0.2.0, cpufeatures 0.2.17, crc32fast 1.5.2, crossbeam-channel 0.5.17, crossbeam-deque 0.8.8, crossbeam-epoch 0.9.21, crossbeam-utils 0.8.23, crypto-common 0.1.7, defmt 1.1.1, defmt-macros 1.1.1, defmt-parser 1.0.0, deranged 0.5.8, digest 0.10.7, dirs 7.0.0, dirs-sys 0.5.0, displaydoc 0.2.7, dtoa 1.0.11, dyn-clone 1.0.20, either 1.19.0, embed_plist 1.2.2, erased-serde 0.4.10, errno 0.3.14, fdeflate 0.3.7, field-offset 0.3.6, flate2 1.1.10, form_urlencoded 1.2.2, futures-channel 0.3.34, futures-core 0.3.34, futures-executor 0.3.34, futures-io 0.3.34, futures-macro 0.3.34, futures-sink 0.3.34, futures-task 0.3.34, futures-util 0.3.34, getrandom 0.3.4, getrandom 0.4.3, glob 0.3.4, hashbrown 0.12.3, hashbrown 0.16.1, hashbrown 0.17.1, hashlink 0.12.2, heck 0.4.1, heck 0.5.0, hex 0.4.3, html5ever 0.39.0, http 1.5.0, httparse 1.10.1, iana-time-zone 0.1.65, iana-time-zone-haiku 0.1.2, idna 1.1.0, ipnet 2.12.2, itoa 1.0.18, jni-sys 0.3.1, jni-sys 0.4.1, jni-sys-macros 0.4.1, js-sys 0.3.106, jsonptr 0.7.1, keyboard-types 0.8.3, libc 0.2.190, lock_api 0.4.14, log 0.4.34, markup5ever 0.39.0, mime 0.3.17, native-tls 0.2.18, ndarray 0.17.2, ndk 0.9.0, ndk-context 0.1.1, ndk-sys 0.6.0+11769913, num-complex 0.4.6, num-conv 0.2.2, num-integer 0.1.47, num-traits 0.2.19, once_cell 1.21.4, openssl-probe 0.2.1, ort 2.0.0-rc.13, ort-sys 2.0.0-rc.13, parking_lot 0.12.5, parking_lot_core 0.9.12, percent-encoding 2.3.2, png 0.17.16, png 0.18.1, powerfmt 0.2.1, primal-check 0.3.4, proc-macro-crate 1.3.1, proc-macro-crate 2.0.2, proc-macro-crate 3.5.0, proc-macro-error 1.0.4, proc-macro-error-attr 1.0.4, proc-macro2 1.0.107, quote 1.0.47, rayon 1.12.0, rayon-core 1.13.0, ref-cast 1.0.27, ref-cast-impl 1.0.27, regex 1.13.1, regex-automata 0.4.18, regex-syntax 0.8.11, reqwest 0.13.5, rustfft 6.4.1, rustls-pki-types 1.15.1, rustversion 1.0.23, scopeguard 1.2.0, security-framework 3.7.0, security-framework-sys 2.17.0, semver 1.0.28, serde 1.0.229, serde-untagged 0.1.9, serde_core 1.0.229, serde_derive 1.0.229, serde_derive_internals 0.29.1, serde_json 1.0.151, serde_repr 0.1.21, serde_spanned 0.6.9, serde_spanned 1.1.2, serde_with 3.24.0, serde_with_macros 3.24.0, serialize-to-javascript 0.1.2, serialize-to-javascript-impl 0.1.2, servo_arc 0.4.3, sha2 0.10.9, siphasher 1.0.4, smallvec 1.16.2, socket2 0.6.5, softbuffer 0.4.8, stable_deref_trait 1.2.1, strength_reduce 0.2.4, string_cache 0.9.0, swift-rs 1.0.8, syn 1.0.109, syn 2.0.119, syn 3.0.6, tao-macros 0.1.4, tempfile 3.27.0, tendril 0.5.1, thiserror 1.0.69, thiserror 2.0.21, thiserror-impl 1.0.69, thiserror-impl 2.0.21, time 0.3.55, time-core 0.1.9, time-macros 0.2.32, toml 1.1.8+spec-1.1.0, toml_datetime 0.6.3, toml_datetime 1.1.2+spec-1.1.0, toml_edit 0.19.15, toml_edit 0.20.2, toml_edit 0.25.17+spec-1.1.0, toml_parser 1.1.5+spec-1.1.0, toml_writer 1.1.3+spec-1.1.0, transpose 0.2.3, tray-icon 0.25.1, typeid 1.0.3, typenum 1.20.1, unicode-segmentation 1.13.3, ureq 3.4.2, ureq-proto 0.6.4, url 2.5.8, utf8-zero 0.8.1, wasm-bindgen 0.2.129, wasm-bindgen-futures 0.4.79, wasm-bindgen-macro 0.2.129, wasm-bindgen-macro-support 0.2.129, wasm-bindgen-shared 0.2.129, wasm-streams 0.5.0, web-sys 0.3.106, web-time 1.1.0, web_atoms 0.2.6, windows 0.62.2, windows-collections 0.3.2, windows-core 0.62.2, windows-future 0.3.2, windows-implement 0.60.2, windows-interface 0.59.3, windows-link 0.2.1, windows-numerics 0.3.1, windows-result 0.4.1, windows-strings 0.5.1, windows-sys 0.45.0, windows-sys 0.60.2, windows-sys 0.61.2, windows-targets 0.42.2, windows-targets 0.53.5, windows-threading 0.2.1, windows-version 0.1.7, windows_aarch64_gnullvm 0.42.2, windows_aarch64_gnullvm 0.53.1, windows_aarch64_msvc 0.42.2, windows_aarch64_msvc 0.53.1, windows_i686_gnu 0.42.2, windows_i686_gnu 0.53.1, windows_i686_gnullvm 0.53.1, windows_i686_msvc 0.42.2, windows_i686_msvc 0.53.1, windows_x86_64_gnu 0.42.2, windows_x86_64_gnu 0.53.1, windows_x86_64_gnullvm 0.42.2, windows_x86_64_gnullvm 0.53.1, windows_x86_64_msvc 0.42.2, windows_x86_64_msvc 0.53.1

### MIT (100)

atk 0.18.2, atk-sys 0.18.2, block2 0.6.2, bytes 1.12.1, cairo-rs 0.18.5, cairo-sys-rs 0.18.2, cargo_metadata 0.19.2, cfb 0.14.0, combine 4.6.8, darling 0.24.1, darling_core 0.24.1, darling_macro 0.24.1, derive_more 2.1.1, derive_more-impl 2.1.1, dlopen2 0.8.2, dlopen2_derive 0.4.3, dom_query 0.28.0, gdk 0.18.2, gdk-pixbuf 0.18.5, gdk-pixbuf-sys 0.18.0, gdk-sys 0.18.2, gdkwayland-sys 0.18.2, gdkx11 0.18.2, gdkx11-sys 0.18.2, generic-array 0.14.7, gio 0.18.4, gio-sys 0.18.1, glib 0.18.5, glib-macros 0.18.5, glib-sys 0.18.1, gobject-sys 0.18.0, gtk 0.18.2, gtk-sys 0.18.2, gtk3-macros 0.18.2, http-body 1.1.0, http-body-util 0.1.5, hyper 1.12.0, hyper-util 0.1.21, ico 0.5.0, infer 0.22.0, javascriptcore-rs 1.1.2, javascriptcore-rs-sys 1.1.1, libredox 0.1.25, libsqlite3-sys 0.38.2, memoffset 0.9.1, mio 1.2.4, new_debug_unreachable 1.0.6, objc2 0.6.5, objc2-encode 4.1.0, objc2-foundation 0.3.2, openssl-sys 0.9.117, pango 0.18.3, pango-sys 0.18.0, phf 0.13.1, phf_generator 0.13.1, phf_macros 0.13.1, phf_shared 0.13.1, plist 1.10.1, precomputed-hash 0.1.1, quick-xml 0.42.0, redox_syscall 0.5.18, redox_users 0.5.3, rfd 0.16.0, rsqlite-vfs 0.1.1, rsworld 0.1.0, rsworld-sys 0.1.0, rusqlite 0.40.2, schannel 0.1.29, schemars 0.8.22, schemars 0.9.0, schemars 1.2.2, schemars_derive 0.8.22, simd-adler32 0.3.10, slab 0.4.12, soup3 0.5.0, soup3-sys 0.5.0, sqlite-wasm-rs 0.5.5, strsim 0.11.1, synstructure 0.14.0, tokio 1.53.2, tokio-util 0.7.19, tower 0.5.3, tower-http 0.6.11, tower-layer 0.3.3, tower-service 0.3.3, tracing 0.1.44, tracing-core 0.1.36, try-lock 0.2.5, urlpattern 0.6.0, want 0.3.2, webkit2gtk 2.0.2, webkit2gtk-sys 2.0.2, webview2-com 0.39.1, webview2-com-macros 0.8.1, webview2-com-sys 0.39.1, winnow 0.5.40, winnow 1.0.4, x11 2.21.0, x11-dl 2.21.0, zmij 1.0.23

### Apache-2.0 OR MIT (32)

atomic-waker 1.1.2, base64ct 1.8.3, bit-set 0.8.0, bit-vec 0.8.0, ctor 1.0.13, der 0.8.2, equivalent 1.0.2, fastrand 2.5.0, idna_adapter 1.2.2, indexmap 1.9.3, indexmap 2.14.2, libappindicator 0.9.0, libappindicator-sys 0.9.0, muda 0.20.0, pem-rfc7468 1.0.0, pin-project-lite 0.2.17, portable-atomic 1.15.0, portable-atomic-util 0.2.8, rustc-hash 2.1.3, tauri 2.12.2, tauri-codegen 2.7.1, tauri-macros 2.7.1, tauri-plugin-dialog 2.8.1, tauri-plugin-fs 2.6.0, tauri-runtime 2.12.1, tauri-runtime-wry 2.12.1, tauri-utils 2.10.1, utf8_iter 1.0.4, uuid 1.27.0, window-vibrancy 0.8.1, wry 0.57.0, zeroize 1.9.1

### MIT/Apache-2.0 (19)

bitflags 1.3.2, bs58 0.5.1, fallible-iterator 0.3.0, fallible-streaming-iterator 0.1.9, foreign-types 0.3.2, foreign-types 0.5.0, foreign-types-macros 0.2.4, foreign-types-shared 0.1.1, foreign-types-shared 0.3.1, ident_case 1.0.1, jni 0.21.1, json-patch 4.2.0, matrixmultiply 0.3.11, openssl-macros 0.1.1, rawpointer 0.2.1, socks 0.3.4, winapi 0.3.9, winapi-i686-pc-windows-gnu 0.4.0, winapi-x86_64-pc-windows-gnu 0.4.0

### Unicode-3.0 (18)

icu_collections 2.3.0, icu_locale_core 2.3.0, icu_normalizer 2.3.0, icu_normalizer_data 2.3.0, icu_properties 2.3.0, icu_properties_data 2.3.0, icu_provider 2.3.1, litemap 0.8.3, potential_utf 0.1.6, tinystr 0.8.4, writeable 0.6.4, yoke 0.8.3, yoke-derive 0.8.4, zerofrom 0.1.8, zerofrom-derive 0.1.8, zerotrie 0.2.5, zerovec 0.11.8, zerovec-derive 0.11.6

### Zlib OR Apache-2.0 OR MIT (17)

bytemuck 1.25.2, dispatch2 0.3.1, objc2-app-kit 0.3.2, objc2-cloud-kit 0.3.2, objc2-core-data 0.3.2, objc2-core-foundation 0.3.2, objc2-core-graphics 0.3.2, objc2-core-image 0.3.2, objc2-core-location 0.3.2, objc2-core-text 0.3.2, objc2-exception-helper 0.1.1, objc2-io-surface 0.3.2, objc2-quartz-core 0.3.2, objc2-ui-kit 0.3.2, objc2-user-notifications 0.3.2, objc2-web-kit 0.3.2, tinyvec 1.13.3

### Unlicense OR MIT (9)

aho-corasick 1.1.5, byteorder 1.5.0, jiff 0.2.38, jiff-core 0.1.1, jiff-static 0.2.38, jiff-tzdb 0.1.9, jiff-tzdb-platform 0.1.3, memchr 2.8.3, winapi-util 0.1.11

### MPL-2.0 (5)

cssparser 0.37.0, cssparser-macros 0.7.1, dtoa-short 0.3.5, option-ext 0.2.0, selectors 0.38.0

### Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT (5)

linux-raw-sys 0.12.1, rustix 1.1.5, wasi 0.11.1+wasi-snapshot-preview1, wasip2 1.0.4+wasi-0.2.12, wit-bindgen 0.57.1

### Apache-2.0 (4)

hound 3.5.1, openssl 0.10.81, sync_wrapper 1.0.2, tao 0.37.1

### Unlicense (3)

midly 0.5.3, whisper-rs 0.16.0, whisper-rs-sys 0.15.0

### Apache-2.0/MIT (3)

cesu8 1.1.0, dbus 0.9.12, libdbus-sys 0.2.7

### MIT OR Zlib OR Apache-2.0 (2)

miniz_oxide 0.8.9, miniz_oxide 0.9.1

### Unlicense/MIT (2)

same-file 1.0.6, walkdir 2.5.0

### MIT OR Apache-2.0 OR LGPL-2.1-or-later (2)

r-efi 5.3.0, r-efi 6.0.0

### Zlib (2)

foldhash 0.2.0, zlib-rs 0.6.8

### BSD-3-Clause OR MIT OR Apache-2.0 (2)

num_enum 0.7.6, num_enum_derive 0.7.6

### BSD-3-Clause (2)

alloc-no-stdlib 3.0.0, alloc-stdlib 0.3.0

### ISC (1)

libloading 0.7.4

### CC0-1.0 OR MIT-0 OR Apache-2.0 (1)

dunce 1.0.5

### CDLA-Permissive-2.0 (1)

webpki-root-certs 1.0.9

### Apache-2.0 / MIT (1)

fnv 1.0.7

### MIT OR Apache-2.0 OR Zlib (1)

raw-window-handle 0.6.2

### 0BSD OR MIT OR Apache-2.0 (1)

adler2 2.0.1

### BSD-3-Clause AND MIT (1)

brotli 9.0.0

### BSD-3-Clause/MIT (1)

brotli-decompressor 6.0.1

### (MIT OR Apache-2.0) AND Unicode-3.0 (1)

unicode-ident 1.0.26

### Apache-2.0 AND MIT (1)

dpi 0.1.2

각 라이선스 전문은 https://spdx.org/licenses/ 에서 볼 수 있습니다. Full license texts: https://spdx.org/licenses/
