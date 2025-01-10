FROM rust:1.83.0-alpine3.21 AS builder

RUN apk add musl-dev libpq-dev

WORKDIR /usr/src/bot
COPY . .

ENV RUSTFLAGS='-C target-feature=-crt-static'
RUN \
    --mount=type=cache,target=~/.cargo/registry/index/ \
    --mount=type=cache,target=~/.cargo/registry/cache/ \
    --mount=type=cache,target=~/.cargo/git/db/ \
    --mount=type=cache,target=target/ \
    cargo build --release && mv target/release/aihasto-bot /usr/bin/aihasto-bot

FROM alpine:3.21

RUN apk add libpq libgcc

WORKDIR /usr/bot
COPY --from=builder /usr/bin/aihasto-bot ./

CMD ["/usr/bot/aihasto-bot"]
