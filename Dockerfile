FROM rust:1.51 as depbuild
# This is a stage with built dependencies and a dummy server project
WORKDIR /usr/src
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./core ./core
# TODO: find a way to avoid adding client to the workspace
COPY ./client ./client

RUN cargo new --bin --name noxious-server server
COPY ./server/Cargo.toml ./server/Cargo.toml

RUN cargo build --release --package noxious-server

RUN rm -f ./target/release/deps/noxious_server*
RUN rm ./target/release/noxious-server*

# -----------

FROM rust:1.51 as serverbuild
WORKDIR /usr/src
COPY --from=depbuild /usr/src/Cargo.toml ./Cargo.toml
COPY --from=depbuild /usr/src/Cargo.lock ./Cargo.lock
COPY --from=depbuild /usr/src/core ./core
COPY --from=depbuild /usr/src/client ./client
COPY --from=depbuild /usr/src/target ./target
COPY ./server ./server

RUN cargo build --release --package noxious-server

# -----------

FROM debian:buster-slim as server

WORKDIR /app
COPY --from=serverbuild /usr/src/target/release/noxious-server ./noxious-server

EXPOSE 8474

ENTRYPOINT ["./noxious-server"]
CMD ["--host=0.0.0.0"]