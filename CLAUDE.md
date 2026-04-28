# capacitor-native-agent — project rules

## The Rust crate is a submodule, not a fork

`rust/native-agent-ffi/` is a **git submodule** pointing at the upstream Rust crate at https://gitlab.k8s.t6x.io/rruiz/native-agent-ffi. This repo stores only a SHA pointer to a commit on that upstream — there is no fork, no vendored copy, no duplicated source.

### Hard rules

- **Do not edit any file under `rust/native-agent-ffi/`.** All Rust changes belong upstream on GitLab. After landing them upstream, advance the submodule pointer here and rebuild.
- **Do not delete the `rust/` directory or the submodule.** `scripts/build-android.sh` and `scripts/build-ios.sh` both `cd` into `rust/native-agent-ffi/` to invoke `cargo`. Without the submodule checked out, neither build can run.
- **Do not commit binaries from `rust/native-agent-ffi/target/`.** It is `.gitignore`d for a reason — only the cross-compiled artifacts end up in `android/src/main/jniLibs/` and `ios/Frameworks/`.

### After cloning this repo

```bash
git submodule update --init --recursive
```

### To pull upstream Rust changes

```bash
cd rust/native-agent-ffi
git fetch origin
git checkout <new-sha>     # or: git pull --ff-only origin main
cd ../..
git add rust/native-agent-ffi
# then rebuild and commit
```

## What ships to npm

The `files` field in `package.json` controls the tarball. Source-of-truth: only the **prebuilt artifacts** ship:

- `dist/esm/` — TypeScript build output
- `android/src/main/` — Kotlin plugin source + generated UniFFI bindings + `.so` under `jniLibs/`
- `android/build.gradle`
- `ios/Sources/` — Swift plugin source + generated UniFFI bindings
- `ios/Frameworks/NativeAgentFFI.xcframework/` — prebuilt staticlibs (device + simulator)
- `CapacitorNativeAgent.podspec`, `Package.swift`, `README.md`, `LICENSE`

The `rust/` submodule, the `example/` smoke-test app, and any `android/build/` Gradle output are **never** included.

## Release flow

1. Pull submodule to the new upstream SHA (see above).
2. **Android rebuild (Linux or Mac):** `scripts/build-android.sh` — runs `cargo ndk -t arm64-v8a build --release`, copies `libnative_agent_ffi.so` into `android/src/main/jniLibs/arm64-v8a/`, regenerates the Kotlin UniFFI binding.
3. **iOS rebuild (Mac only):** `scripts/build-ios.sh` — builds device + simulator staticlibs, regenerates the Swift UniFFI binding, assembles `ios/Frameworks/NativeAgentFFI.xcframework/`.
4. `npm run clean && npm run build` — TypeScript build.
5. Bump version in `package.json`. There are no other version refs in the tree.
6. `npm pack --dry-run` — verify tarball contents (no `rust/`, no `example/`, no `android/build/`).
7. Commit (include the submodule pointer move), tag `vX.Y.Z`, push to GitHub.
8. `npm publish`.

## Mac remote convention

iOS staticlibs cannot be cross-compiled from Linux. iOS work happens on the project's Mac remote (`rogelioruizgatica@10.61.192.207`) **strictly under `~/choreruiz/`** — never elsewhere in the Mac home. This rule comes from the parent repo's `CLAUDE.md` and applies to this project too.

Typical iOS rebuild session on the Mac:

```bash
cd ~/choreruiz/capacitor-native-agent       # clone or rsync first if missing
git pull && git submodule update --init --recursive
(cd rust/native-agent-ffi && git fetch && git checkout <sha>)
scripts/build-ios.sh
```

Then rsync `ios/Frameworks/NativeAgentFFI.xcframework/` and `ios/Sources/NativeAgentPlugin/Generated/` back to the Linux dev host.

## ⚠️ Known release-state drift: iOS stale in `0.9.0`

`0.9.0` was cut Android-only because the Mac build host (`rogelioruizgatica@10.61.192.207`) was unreachable. **The shipped `ios/Frameworks/NativeAgentFFI.xcframework/` was NOT rebuilt against `native-agent-ffi@9946e0f`** — it is still the prior `0.8.x` build. The Android `.so` IS up to date.

Concretely missing on iOS until `0.9.1`:

- Kimi provider — `provider: 'kimi' | 'kimi-coding' | 'kimi-code'` returns "Unsupported provider" on iOS.
- SSE `data:`-no-space tolerance — Anthropic-compatible streaming endpoints (e.g. Kimi) silently produce empty messages on iOS.
- `send_message` session-context fix — first turn after cold-boot/resume may drop prior history on iOS.

(`AgentStore` trait extraction and runtime-drop fix do not affect Capacitor — internal refactor and server-only respectively.)

### Action when Mac is reachable

```bash
ssh rogelioruizgatica@10.61.192.207
cd ~/choreruiz/capacitor-native-agent           # clone or rsync first if missing
git pull && git submodule update --init --recursive
(cd rust/native-agent-ffi && git fetch && git checkout 9946e0f)
scripts/build-ios.sh
```

Then back on Linux: rsync `ios/Frameworks/NativeAgentFFI.xcframework/` and `ios/Sources/NativeAgentPlugin/Generated/{native_agent_ffi.swift,native_agent_ffiFFI.h}`, bump `package.json` to `0.9.1`, commit "fix: rebuild iOS xcframework against native-agent-ffi@9946e0f", tag `v0.9.1`, push, `npm publish`.

## Supported LLM providers

`provider` is a free `string` across the JS → Kotlin/Swift → FFI boundary. Accepted values are decided in the Rust crate's `agent_loop::create_driver`. Currently:

- `anthropic` — Claude
- `openai` — OpenAI
- `openrouter` — OpenRouter
- `kimi` (aliases `kimi-coding`, `kimi-code`) — Kimi Coding via Anthropic-messages-compatible endpoint

To add a new provider, the change goes upstream in the Rust crate, then this repo's submodule pointer advances.
