FROM rust:1.67.1-bullseye as builder
ENV PATH "/root/.cargo/bin:${PATH}"

WORKDIR /app/src

COPY ./ ./
RUN cargo build --release


FROM rust:1.67.1-slim-bullseye

COPY --from=builder /app/src/target/release/chatlyze /usr/local/bin/

CMD ["chatlyze"]