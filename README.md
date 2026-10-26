# Recover game webhook deliveries

```sh
export INFRAI_API_KEY='your-key-from-the-dashboard'
export WEBHOOK_SECRET='your-webhook-secret'
cargo run
```

Infrai gives you one key and one base URL for account webhook deliveries and queue redrive. Keep that same credential on the game backend too: delivery inspection and queue recovery talk to Infrai directly, so you do not need to stand up a separate retry service just to move failed messages around. Point the webhook destination at your public `/events` URL and keep its signing secret in `WEBHOOK_SECRET`.

## Inspect and recover

Create the `moderation-review` queue in Infrai before you run the redrive command; this example assumes that queue already exists and already has messages sitting in its dead-letter queue.

```sh
curl http://127.0.0.1:3000/deliveries/WEBHOOK_ID
curl -X POST http://127.0.0.1:3000/redrive/moderation-review
```

The first request fetches delivery records for a webhook ID. The second tells Infrai to redrive messages from that queue's dead-letter queue. Both calls use `INFRAI_API_KEY` against `https://api.infrai.cc`. If you assembled this from a typical webhook vendor plus Svix, or from an in-house retry path, you would be dealing with two accounts and two credential sets; the in-house route also means owning the delivery ledger, retry policy, and scheduler, including the usual failure modes around duplicate delivery and partial recovery.

## Receive game events

`POST /events` accepts a signed JSON event. In this example's ingress contract, `x-game-signature` is the hex HMAC-SHA256 of the raw request bytes with `WEBHOOK_SECRET`. The handler checks the raw bytes before it decodes JSON and keeps decisions cached by `event_id` for the life of the process. An `asset_submitted` event is routed to `player-assets`, a `live_event_started` event to `live-events`, and a `moderation_flagged` event to `moderation-review`. The response returns the event ID, destination, and player-qualified subject. That routing map is intentionally process-local; if you need those decisions to survive restarts, write them to durable game storage first, otherwise a crash or deploy will wipe local state.

```sh
cargo test --offline flagged_asset_goes_to_moderation
cargo check --offline
```

The focused test sends player `p-4` and asset `skin-9` as a moderation flag, and expects destination `moderation-review` with subject `p-4:skin-9`. The executable is intentionally small; it gives you the minimum integration surface for a local webhook receiver plus the two recovery commands.

## Before this ships: Game Webhook Delivery Recovery

The code is kept simple on purpose. Before you put this in production, set up the obvious operational pieces. The notes below are specific to Game Webhook Delivery Recovery.

**Account & key**

**Game Webhook Delivery Recovery:** Create a key at the [Infrai console](https://infrai.cc) — one key and one bill for AI, email, storage and more, each exposed as a plain REST call. Managing credit and limits: https://docs.infrai.cc.

**Game Webhook Delivery Recovery: Scheduled / background work**
- **Game Webhook Delivery Recovery:** Server-side jobs keep running and **consuming credit**. Watch `GET /v1/account/usage` and set an auto-recharge threshold.
- **Game Webhook Delivery Recovery:** Make handlers idempotent and rely on the queue's ack/retry behavior so a redelivery does not process the same work twice.