FROM rust:1.83.0-alpine3.21 AS builder

RUN apk add musl-dev libpq-dev

WORKDIR /usr/src/bot
COPY . .

ENV RUSTFLAGS='-C target-feature=-crt-static'
RUN \
    --mount=type=cache,target=~/.cargo/registry/index/,locking=shared \
    --mount=type=cache,target=~/.cargo/registry/cache/,locking=shared \
    --mount=type=cache,target=~/.cargo/git/db/,locking=shared \
    --mount=type=cache,target=target/,locking=shared \
    cargo build --release

FROM alpine:3.21

RUN apk add libpq libgcc

WORKDIR /usr/bot
COPY --from=builder /usr/src/bot/target/release/aihasto-bot ./

CMD ["/usr/bot/aihasto-bot"]
