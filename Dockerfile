FROM messense/rust-musl-cross:x86_64-musl as builder

WORKDIR /touchid

COPY ./Cargo.toml ./Cargo.toml
RUN ls ./Cargo.lock && cp ./Cargo.lock ./ || true
COPY ./src ./src

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder touchid/target/x86_64-unknown-linux-musl/release/touchid /touchid
ENTRYPOINT ["/touchid"]
EXPOSE 3000