FROM rust:1-slim AS build
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /app/target/release/cronogramando-backend /app/backend
EXPOSE 8000
CMD ["/app/backend"]
