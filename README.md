# capacitor-native-agent

Native AI agent loop for Capacitor apps. Runs LLM completions, tool execution, cron jobs, and session persistence in native Rust via UniFFI, enabling true background execution on mobile.

## Features

- **Native agent loop** — LLM streaming, multi-turn tool calling, abort/steer
- **Built-in tools** — File I/O, git (libgit2), grep, shell exec, fetch
- **SQLite persistence** — Sessions, cron jobs, skills, scheduler/heartbeat config
- **Background execution** — Runs outside the WebView lifecycle (WorkManager / BGProcessingTask)
- **Auth management** — API key and OAuth token storage with refresh
- **Cron & heartbeat** — Scheduled agent runs with wake evaluation
- **MCP support** — Model Context Protocol server integration
- **Event bridge** — Streams events (text_delta, tool_use, tool_result, etc.) to the WebView

## Install

```bash
npm install capacitor-native-agent
npx cap sync
```

## Cloning this repository

The Rust FFI crate lives in a separate GitLab repository and is pulled in as a git submodule. After cloning:

```bash
git submodule update --init --recursive
```

Upstream URL: https://gitlab.k8s.t6x.io/rruiz/native-agent-ffi

## Android Setup

The npm package includes the Kotlin plugin source and UniFFI bindings, but **not** the compiled Rust shared library. You must build and place it yourself:

### 1. Build the Rust .so

```bash
scripts/build-android.sh
```

This runs `cargo ndk -t arm64-v8a build --release` on the submodule and copies the result into `android/src/main/jniLibs/arm64-v8a/`.

### 2. Place the .so in your app

Copy the built library to your Android app's jniLibs:

```bash
cp target/aarch64-linux-android/release/libnative_agent_ffi.so \
   <your-app>/android/app/src/main/jniLibs/arm64-v8a/
```

### 3. Sync and build

```bash
npx cap sync android
cd android && ./gradlew assembleDebug
```

## Usage

```typescript
import { NativeAgent } from 'capacitor-native-agent'

// Listen for agent events
NativeAgent.addListener('nativeAgentEvent', (event) => {
  const { eventType, payloadJson } = event
  const payload = JSON.parse(payloadJson)

  if (eventType === 'text_delta') {
    process.stdout.write(payload.text)
  }
})

// Initialize
await NativeAgent.initialize({
  dbPath: 'files://agent.db',
  workspacePath: '/path/to/workspace',
  authProfilesPath: '/path/to/auth-profiles.json',
})

// Set auth
await NativeAgent.setAuthKey({
  key: 'sk-ant-...',
  provider: 'anthropic',
  authType: 'api_key',
})

// Send a message
const { runId } = await NativeAgent.sendMessage({
  prompt: 'Hello!',
  sessionKey: 'session-1',
  systemPrompt: 'You are a helpful assistant.',
})
```

## API

See [definitions.ts](src/definitions.ts) for the full TypeScript interface.

### Core Methods

| Method | Description |
|--------|-------------|
| `initialize()` | Create the native agent handle |
| `sendMessage()` | Start an agent turn |
| `followUp()` | Continue the conversation |
| `abort()` | Cancel the running turn |
| `steer()` | Inject guidance into a running turn |

### Auth

| Method | Description |
|--------|-------------|
| `getAuthToken()` | Get stored auth token |
| `setAuthKey()` | Store API key or OAuth token |
| `deleteAuth()` | Remove auth for a provider |
| `refreshToken()` | Refresh an OAuth token |
| `getAuthStatus()` | Get masked key status |

### Sessions

| Method | Description |
|--------|-------------|
| `listSessions()` | List all sessions |
| `loadSession()` | Load session message history |
| `resumeSession()` | Resume a previous session |
| `clearSession()` | Clear current session |

### Cron & Scheduling

| Method | Description |
|--------|-------------|
| `addCronJob()` | Create a scheduled job |
| `updateCronJob()` | Update job config |
| `removeCronJob()` | Delete a job |
| `listCronJobs()` | List all jobs |
| `runCronJob()` | Force-trigger a job |
| `handleWake()` | Evaluate due jobs (called from WorkManager) |
| `getSchedulerConfig()` | Get scheduler + heartbeat config |
| `setSchedulerConfig()` | Update scheduler config |
| `setHeartbeatConfig()` | Update heartbeat config |

### Tools

| Method | Description |
|--------|-------------|
| `invokeTool()` | Execute a tool directly |
| `startMcp()` | Start MCP server |
| `restartMcp()` | Restart MCP with new tools |

## Event Types

Events are emitted via `addListener('nativeAgentEvent', handler)`:

- `text_delta` — Streaming text chunk
- `thinking` — Model thinking content
- `tool_use` — Tool invocation started
- `tool_result` — Tool completed
- `agent.completed` — Turn finished with usage stats
- `agent.error` — Error occurred
- `approval_request` — Tool needs user approval
- `cron.job.started` / `cron.job.completed` / `cron.job.error` — Cron lifecycle
- `heartbeat.*` — Heartbeat lifecycle
- `scheduler.status` — Scheduler state updates

## Supported LLM providers

The `provider` argument is a free string. The Rust agent loop currently accepts:

| Provider string | Default model | Endpoint |
|---|---|---|
| `anthropic` | `claude-sonnet-4-20250514` | Anthropic Messages API |
| `openai` | `gpt-4o` | OpenAI Chat Completions |
| `openrouter` | `anthropic/claude-sonnet-4.5` | OpenRouter |
| `kimi` (aliases `kimi-coding`, `kimi-code`) | `kimi-for-coding` | Kimi Coding (Anthropic-messages-compatible, https://api.kimi.com/coding) |

> **Note (0.9.0 — iOS):** Kimi is fully wired in the Android `.so` shipped with this version. The iOS `xcframework` in this release is the pre-existing `0.8.x` build and has **not** been rebuilt against `native-agent-ffi 9946e0f`. iOS callers using `provider: 'kimi'` will receive `Unsupported provider`. Track [#TODO-ios-0.9.1](#known-limitations) for the 0.9.1 iOS rebuild.

## Platform Support

| Platform | Status (this release: **0.9.0**) |
|----------|-----------------------------------|
| Android  | Supported. Rust pinned to `native-agent-ffi@9946e0f` (Kimi, SSE no-space fix, session-context fix, AgentStore refactor, runtime-drop fix). |
| iOS      | Stale binary — `xcframework` is pre-`0.9.0` and will be refreshed in `0.9.1`. See *Known limitations* below. |
| Web      | N/A (throws unavailable error). |

## Known limitations

### 0.9.0 — iOS xcframework not rebuilt

The Mac build host was unreachable when `0.9.0` was cut, so `ios/Frameworks/NativeAgentFFI.xcframework/` was **not** rebuilt against `native-agent-ffi@9946e0f`. iOS users on `0.9.0` are missing the following upstream changes (Android users have them):

- `ba8c97e` `feat(llm): add kimi (Kimi Code) provider` — `provider: 'kimi'` will fail on iOS until `0.9.1`.
- `2cb34be` `fix(llm): tolerate "data:" SSE without space` — Anthropic-compatible streaming endpoints that omit the space after `data:` will return empty messages on iOS.
- `9735f4d` `ffi: keep session context across send_message` — after a cold-boot resume, the next `sendMessage` may start without prior history on iOS.
- `02b1955` `feat(store): extract AgentStore trait` — internal refactor only; mobile `SqliteStore` default behavior unchanged, so no user-visible iOS impact.
- `9946e0f` `ffi: detach inner runtime on drop` — server-side concern only (handle dropped from inside another tokio runtime); does not affect Capacitor.

Action plan: rebuild `xcframework` on the Mac under `~/choreruiz/` via `scripts/build-ios.sh`, rsync `ios/Frameworks/` and `ios/Sources/NativeAgentPlugin/Generated/` back, bump to `0.9.1`, republish.

## License

MIT
