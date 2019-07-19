FROM rust:stretch as builder

RUN apt-get update && \
	apt-get install -y --no-install-recommends \
	make cmake g++ gcc

RUN mkdir /zebra
WORKDIR /zebra

ENV RUST_BACKTRACE 1
ENV CARGO_HOME /zebra/.cargo/

# Copy cached target/ from master
# COPY --from=gcr.io/zebrad/master:latest /zebra/target /zebra/target

# Copy cached .cargo/ from master
# COPY --from=gcr.io/zebrad/master:latest /zebra/.cargo /zebra/.cargo

# Copy local code to the container image.
# Assumes that we are in the git repo.

COPY . .

RUN cargo fetch --verbose

COPY . .

RUN cargo test --all && cargo build --release

FROM debian:latest

COPY --from=builder /zebra/target/release/zebrad /zebrad

CMD ["/zebrad", "--data-dir=./.zebra-testnet", " --testnet"]
