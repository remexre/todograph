FROM clux/muslrust:latest
WORKDIR /usr/src/todograph
COPY . .
RUN cargo build --release

FROM scratch
USER nobody
COPY --from=0 /usr/src/todograph/target/x86_64-unknown-linux-musl/release/todograph /todograph
CMD /todograph

# vi:syntax=dockerfile
