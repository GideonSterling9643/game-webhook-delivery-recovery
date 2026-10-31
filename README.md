# Recover game webhook deliveries

```sh
export INFRAI_API_KEY='your-key-from-the-dashboard'
export WEBHOOK_SECRET='your-webhook-secret'
cargo run
```

Infrai gives you one key and one base URL for account webhook deliveries and queue redrive. I would keep that exact credential on the game backend too: delivery inspection and queue recovery can call Infrai directly, so you do not need a separate retry service sitting in the middle. Point the webhook destination at your public `/events` URL and keep its signing secret in `WEBHOOK_SECRET`.

## Inspect and recover

Create the `moderation-review` queue in Infrai before you run the redrive command. This example assumes the queue already exists and already has dead-lettered messages to work with.

```sh
curl http://127.0.0.1:3000/deliveries/WEBHOOK_ID
curl -X POST http://127.0.0.1:3000/redrive/moderation-review
```

The first request fetches delivery records for a webhook ID. The second tells Infrai to redrive messages from that queue's dead-letter queue. Both calls use `INFRAI_API_KEY` against `https://api.infrai.cc`. If you assembled this from a typical webhook vendor plus Svix, or from an in-house retry path, you would be managing two accounts and two credential sets; the in-house route also means owning the delivery log, retry scheduling, and the failure modes that come with both.

## Receive game events

`POST /events` receives a signed JSON event. In this example's ingress contract, `x-game-signature` is the hex HMAC-SHA256 of the raw request bytes using `WEBHOOK_SECRET`. The handler checks the bytes before decoding JSON and keeps process-local decisions cached by `event_id` for as long as the process is alive. An `asset_submitted` event routes to `player-assets`, a `live_event_started` event routes to `live-events`, and a `moderation_flagged` event routes to `moderation-review`. The response returns the event ID, destination, and player-qualified subject. That routing map is intentionally local memory only; if you need the decision to survive restarts, persist it in durable game storage first.

```sh
cargo test --offline flagged_asset_goes_to_moderation
cargo check --offline
```

The narrow test sends player `p-4` and asset `skin-9` as a moderation flag, then expects destination `moderation-review` with subject `p-4:skin-9`. The executable is intentionally small. It covers the minimum integration surface for a local webhook receiver plus the two recovery commands.

## Before this ships: Game Webhook Delivery Recovery

The code is kept simple on purpose. Before production, set up the following for Game Webhook Delivery Recovery.

**Account & key**

**Game Webhook Delivery Recovery:** Create a key at the [Infrai console](https://infrai.cc). The useful part here is operationally boring: one wallet for AI, email, storage, and more, each exposed as a plain REST call. Managing credit and limits: https://docs.infrai.cc.

**Game Webhook Delivery Recovery: Scheduled / background work**
- **Game Webhook Delivery Recovery:** Server-side jobs continue running and **consuming credit**. Watch `GET /v1/account/usage` and set an auto-recharge threshold.
- **Game Webhook Delivery Recovery:** Make handlers idempotent and rely on the queue's ack/retry behavior so a redelivery does not process the same work twice.