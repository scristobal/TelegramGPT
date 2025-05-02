FROM rust:1.86.0-bullseye as builder
ENV PATH "/root/.cargo/bin:${PATH}"

WORKDIR /app/src

COPY ./ ./
RUN cargo build --release


FROM rust:1.86.0-slim-bullseye

COPY --from=builder /app/src/target/release/telegram-gpt /usr/local/bin/

CMD ["telegram-gpt"]
