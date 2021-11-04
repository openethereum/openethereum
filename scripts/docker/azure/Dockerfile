FROM ubuntu:20.04 as builder
WORKDIR /build

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND="noninteractive" TZ="America/Los_Angeles" apt-get install -y tzdata && \
	apt-get --yes install build-essential cmake libudev-dev curl file && \
	apt-get clean

# install rustup

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
# rustup directory
ENV PATH /root/.cargo/bin:$PATH

RUN rustup override set 1.52.0

# show tools
RUN rustc -vV && cargo -V

# build OpenEthereum
ADD . /build/openethereum

RUN cd openethereum && \
		cargo build --release --features final && \
		chmod +x /build/openethereum/target/release/openethereum

FROM ubuntu:20.04
RUN apt-get update && \
	apt-get clean
COPY --from=builder /build/openethereum/target/release/openethereum /usr/local/bin

EXPOSE 8080 8545 8180
ENTRYPOINT ["/usr/local/bin/openethereum"]
