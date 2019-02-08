FROM rust:latest
WORKDIR /usr/src/todograph
COPY . .
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y ca-certificates libpq5 && rm -rf /var/lib/apt/lists/*
COPY --from=0 /usr/src/todograph/target/release/todograph /usr/local/bin/todograph

USER nobody
CMD /usr/local/bin/todograph

# vi:syntax=dockerfile
