FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml ./
COPY src ./src
RUN cargo test && cargo build --release

FROM debian:bookworm-slim
COPY --from=build /src/target/release/api /usr/local/bin/api
USER nobody
ENV PORT=8080
EXPOSE 8080
CMD ["api"]
