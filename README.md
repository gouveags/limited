# limited

A simple rate limiter app.

## Why I'm building this

This is my personal attempt to build rate limiters while studying system design.

The idea is simple: let a caller do around 500 requests per minute, then say no for a bit. Behind that tiny "no", though, there is a lot worth learning: Redis, atomic operations, Lua scripts, request windows, buckets, bursts, edge cases, and why distributed systems enjoy making simple counters suspicious.

I'm doing this because rate limiting is one of those boxes that shows up in every system design diagram, usually somewhere near the API gateway, looking very calm.

I want to build the thing, break the thing, test the thing, and hopefully understand why each strategy is useful and where each one gets weird.

Rust because I want to learn it properly.

Also because apparently I looked at Redis, Lua, async code, Docker, and edge-case-heavy tests and thought: "yes, let's make the compiler supervise the whole situation."

Anyways...

## Goal

Build a real Redis-backed API that implements several rate limiting strategies behind a clean architecture shape.

## Status

First implementation.

## Strategies

All strategies currently use a limit of `500` requests per `60` seconds.

- `fixed-window`: simple Redis counter per minute-sized window. Easy to understand, but it allows a burst at the window boundary.
- `sliding-window-log`: Redis sorted set of request timestamps. More precise, but stores one entry per request in the active window.
- `token-bucket`: starts full, spends one token per request, and refills over time. Good for controlled bursts.
- `leaky-bucket`: tracks bucket level and leaks over time. Good for smoothing traffic, but still allows a full initial bucket burst.

The multi-command limiter logic runs in Redis Lua scripts so each check-and-update is atomic.

## Run

```bash
docker compose up --build
```

The API listens on `http://localhost:8080`.

## Test

Start Redis:

```bash
docker compose up -d redis
```

Then run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Or:

```bash
./scripts/check.sh
```

## API

Health:

```bash
curl http://localhost:8080/health
```

Increment a counter through a selected limiter:

```bash
curl -X POST http://localhost:8080/limiters/fixed-window/counters/demo/increment
```

Available limiter names:

```text
fixed-window
sliding-window-log
token-bucket
leaky-bucket
```

Read a counter:

```bash
curl http://localhost:8080/counters/demo
```

When the selected limiter rejects a request, the API returns `429 Too Many Requests` with a `Retry-After` header.
