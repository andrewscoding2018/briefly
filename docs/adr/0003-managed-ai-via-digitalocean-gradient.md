# ADR 0003: Managed AI via DigitalOcean Gradient

## Status

Accepted with deferred operational details

## Context

Briefly's long-term product direction includes intent extraction, summarization, and action-item generation, but the app should not own GPU operations or require self-hosted inference infrastructure at the MVP stage. The brief positions DigitalOcean Gradient as the managed inference layer and favors strong open-weight models for structured extraction.

## Decision

Reserve DigitalOcean Gradient as the managed inference provider for future intent extraction and summarization.

## Consequences

- The app can keep AI infrastructure operationally simple in later phases.
- The model layer can target structured extraction and summarization without changing the local-first core architecture.
- AI remains optional and must fail safely without breaking Phase 1 behavior.
- Provider-specific operational choices still need follow-up specification.

## Rejected Alternatives

- Self-hosted inference from day one, because it increases operational burden before the product needs it.
- OpenAI-first or Anthropic-first lock-in, because the baseline architecture should preserve the brief's open-weight, managed-provider direction.

## Deferred Details

- Exact endpoint style
- Model selection fallback policy
- Telemetry and observability vendor
