# Client authentication

`llmap` deliberately reuses credentials clients already possess. A request is
authenticated when its Bearer token or `x-api-key` exactly matches the client
authentication credential of any active configured account. Authentication
grants access to the pool; it does not force routing through the matching
account.

The encrypted client credential and encrypted upstream credential begin with
the same value but have separate lifecycles. An internal OAuth refresh changes
only the upstream credential, so an already configured client is not logged out
when `llmap` refreshes on its behalf. An explicit administrator rotation or an
external credential sync changes both and retains the previous client token for
at most ten minutes. Pausing or deleting the account still invalidates both the
current and handover token immediately.

Comparisons use keyed digests and constant-time equality. Unknown, missing, and
expired tokens have the same external failure. Sending different Bearer and
`x-api-key` values is rejected. Pausing, deleting, or rotating an account
invalidates its old credential at the next request.

## Modes

- `off` does not inspect credential state. Use only on an independently secured
  local network or for controlled migration testing.
- `observe` allows requests but records whether authentication would fail. This
  is the recommended rollout mode.
- `enforce` rejects unknown tokens with `401`. If credential state is
  unavailable it fails closed with `503`, distinguishing outage from bad auth.

## Rollout

Run observe mode long enough to identify every real caller, correct their
credential source, then enable enforce through configuration or
`LLMAP_AUTH_MODE=enforce`. Never derive a second long-lived proxy password from
a provider token or return account-token inventories through the API.

Admin authentication is separate: Argon2id password verification, rate
limiting, opaque HttpOnly SameSite cookies, idle and absolute expiry, and CSRF
tokens protect the branded control plane. HTTP Basic authentication is not used.
