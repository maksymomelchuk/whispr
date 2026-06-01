# Multi-provider AI cleanup: Anthropic native, everyone else OpenAI-compatible

Adding OpenAI, Google, Groq, DeepSeek, Cerebras, OpenRouter, and a user-defined Custom
endpoint as cleanup [[AI Provider]]s alongside the existing Anthropic integration forced a
transport-layer decision: route every provider through one client, or keep Anthropic on a
separate path.

## Decision

**Anthropic is reached through its native Messages API (`/v1/messages`); every other
provider goes through one shared OpenAI-compatible `/chat/completions` client.**

The cleanup ruleset (`effective_prompt()` — safety preamble + rules) is the single shared
source of truth across all providers. Only the request _envelope_ differs per transport:

- **Native (Anthropic):** system prompt as a `system` block array with
  `cache_control: ephemeral`; the OAuth auth mode additionally injects the "Claude Code"
  identity block and the `oauth-2025-04-20` beta header.
- **OpenAI-compatible (all others):** `messages: [{role: system}, {role: user}]`, no cache
  flag, `Authorization: Bearer <key>` (omitted entirely for the Custom provider when its
  key is blank).

## Why not route Anthropic through the OpenAI-compatible path too

Anthropic _does_ publish an OpenAI-compatible endpoint
(`https://api.anthropic.com/v1/chat/completions`), so unifying Anthropic-with-API-key onto
the shared client looks tempting. Two facts kill it:

1. **OAuth pins us to the native path regardless.** A Claude Pro/Max [[OAuth credential]]
   (`sk-ant-oat…`) authenticates only against `/v1/messages` (Bearer + OAuth beta header +
   identity assertion). The compat endpoint accepts an API key, not an OAuth token. So the
   native transport must exist no matter what — routing the API-key case through compat
   would _split_ Anthropic across two transports by auth mode, which is more branching, not
   less.

2. **The compat endpoint drops prompt caching.** Anthropic's docs state prompt caching is
   available only on `/v1/messages`; the OpenAI compatibility layer does not support it and
   is positioned for "testing and comparison," not production. Cleanup runs on _every_
   qualifying dictation with a ~1,600-line system prompt, mostly within the 5-minute cache
   window. Native caching means paying full price for that prefix once, then ~10% per
   subsequent cleanup; the compat path would pay full input price for the whole ruleset on
   every dictation, plus the latency of reprocessing it. Anthropic is the default provider,
   so this hits the common path.

## Considered options

- **OpenRouter-only (one broker, one key).** Tempting for setup simplicity (single key,
  one catalog). Rejected: a broker would see 100% of the user's dictation (a privacy
  regression for a dictation app), it can't carry Anthropic OAuth, and it forces users with
  existing OpenAI/Gemini keys to fund a separate account. The shared OpenAI-compatible
  client makes OpenRouter addable as just one config row if ever wanted, without forcing
  every transcript through it.
- **Anthropic-API-key via the compat endpoint, OAuth native.** Rejected for the two reasons
  above — splits Anthropic across two transports and loses prompt caching on the default
  provider for no code savings.
- **Anthropic native, everyone else OpenAI-compatible.** **Accepted.** Anthropic stays
  unified under one transport (both auth modes), caching intact; one shared client covers
  every other provider; adding a provider is a config row (base URL + curated model list),
  no new transport code.

## Consequences

- The native Anthropic transport and the OpenAI-compatible transport coexist permanently;
  the abstraction boundary is "build the request envelope," with shared prompt content.
- Prompt caching is an Anthropic-only optimization. Other providers rely on whatever
  automatic server-side caching the vendor offers (OpenAI) or none.
- Token-usage stats are a single aggregate: OpenAI-compatible `prompt_tokens` /
  `completion_tokens` map to input / output, with no cache breakdown, and missing `usage`
  is treated as zero rather than failing cleanup.
- Adding Azure (deployment names + `api-version`), AWS Bedrock (SigV4), or Straico
  (non-standard wrapper) would each need work beyond a config row and is out of scope.

## Sources

- [OpenAI SDK compatibility — Claude API Docs](https://docs.anthropic.com/en/api/openai-sdk)
- [Claude prompt caching requires the native Messages format](https://help.apiyi.com/en/claude-prompt-caching-anthropic-native-format-guide-en.html)
