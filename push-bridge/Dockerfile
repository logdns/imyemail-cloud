FROM rust:1.85-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked || cargo build --release

FROM debian:bookworm-slim
COPY --from=build /src/target/release/push-bridge /usr/local/bin/push-bridge
ENV IMYEMAIL_CLOUD_BRIDGE_BIND=0.0.0.0:8787
EXPOSE 8787
CMD ["push-bridge"]
