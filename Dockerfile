FROM clux/muslrust:latest
WORKDIR /usr/src/todograph
COPY . .
RUN cargo build --release

FROM scratch
VOLUME /data
COPY --from=0 /usr/src/todograph/target/x86_64-unknown-linux-musl/release/todograph /todograph
ENTRYPOINT ["/todograph"]
CMD ["--db", "/data/todograph.db", "--port", "80"]

# vi:syntax=dockerfile
