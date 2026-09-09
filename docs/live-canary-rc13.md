# RC13 live canary evidence

This record captures the first post-restart live canary for
`v0.1.0-rc.13`. It is deliberately metadata-only: no provider credential,
administrator credential, prompt body, response body beyond the synthetic
expected word, or proxy userinfo is retained here.

## Subject and rollout

- Run date: 2026-09-09 UTC.
- Public source commit:
  `72dd958478e05f7d0d43a2e0b9b083c14b8d9085`.
- Immutable image:
  `ghcr.io/iggygg/llm-multiaccount-proxy@sha256:848234bc47621a6e0569269d45cebef4035df61d6cf833204000cf1174c565ca`.
- GitOps rollout commit:
  `6cf7a820bbb4d5bf9790ba7cba52daf963eb55df`.
- Public endpoint: `https://llmproxy.triform.wtf`.

Flux applied the exact rollout commit. The StatefulSet then created a fresh
pod from the immutable RC13 digest. Its credential-migration init container
exited zero, both long-running containers became ready with zero restarts, and
the credential watcher reported 11 unchanged credentials. `/health` reported
the RC13 source commit and `/ready` reported ready.

## Persistent account state

The fresh pod reopened the existing encrypted SQLite state and returned this
redacted inventory through the authenticated administrator API:

| Measure | Result |
| --- | ---: |
| Accounts | 11 |
| Enabled / paused | 3 / 8 |
| Claude OAuth | 9 |
| Anthropic API key | 1 |
| Amazon Bedrock | 1 |
| Credential present | 11 of 11 |
| Account egress configured | 0 |

This proves persistence across the RC12-to-RC13 pod replacement. It is not a
backup-restore drill and does not close that separate GA gate.

## Authentication and control-plane checks

- Branded administrator login returned 200 and created an HttpOnly session.
- The authenticated session reported `auth_mode=enforce`.
- A state-changing logout without CSRF returned 403; the same request with the
  session CSRF token returned 204; the invalidated session then returned 401.
- Missing, unknown, and paused-account client credentials each returned 401.
- The account API exposed no credential values or proxy userinfo.

## Real provider journey

An active configured account credential authenticated a synthetic
Anthropic-compatible request for `claude-haiku-4-5-20251001`. The non-streaming
request returned 200 and exactly `OK`. A second request on the same sticky
session returned 200 and a complete stream containing `message_start`,
`content_block_delta`, and `message_stop`.

The two metadata audit events had one account ID, one provider, one hashed
session identifier, and status 200. Neither the client credential nor the
synthetic prompt appeared in the audit response.

The legacy service remained healthy during the run, preserving the rollback
surface. This test did not send duplicate production prompts or move existing
clients from the legacy endpoint.

## Browser journey

A fresh local Chrome session loaded the branded login page, signed in, and
opened the routing control plane. It observed 11 account cards and 3 active
accounts. The add-account dialog exposed Claude OAuth, Anthropic API key,
Bedrock API key, Bedrock SigV4, and Anthropic-compatible providers together
with the ordered residential-proxy-chain field and its no-direct-fallback
contract.

The document used an inline branded SVG favicon, so the browser made no
`/favicon.ico` request. Login and dashboard requests returned 200 and the
browser console reported zero errors or warnings.

## Verdict and remaining boundaries

The RC13 restart, authentication, administrator, routing, streaming, sticky,
redaction, and browser canary is **PASS** for this synthetic journey. RC13 is
still a release candidate, not GA:

- no real residential proxy is configured or vendor-qualified in this canary;
- a live encrypted backup restore drill remains open;
- load, long-stream, disconnect, mutation, disk-pressure, and soak
  qualification remain open;
- a deliberate client rollback rehearsal and 30 consecutive clean canary days
  remain open;
- independent penetration testing, support ownership, SLOs, and on-call
  readiness remain open; and
- the documented single-node SQLite boundary is not a multi-replica HA claim.

