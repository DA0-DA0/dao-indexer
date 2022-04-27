FROM cosmwasm/wasmd:v0.18.0 as wasmd
FROM cosmwasm/rust-optimizer:0.11.5 as rust-optimizer
FROM ubuntu:20.04
# FROM postgres:14


COPY --from=wasmd /usr/bin/wasmd /usr/local/bin/wasmd
COPY --from=wasmd /opt/* /opt/

RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates curl file \
    build-essential \
    git \
    gcc \
    jq \
    musl-tools \
    wget \
    gnupg \ 
    libssl-dev \ 
    pkg-config \
    autoconf automake autotools-dev libtool xutils-dev

ARG DEBIAN_FRONTEND=noninteractive
# RUN apt-get update -y -qq --fix-missing
# RUN apt-get install -y wget gnupg
RUN echo "deb http://apt.postgresql.org/pub/repos/apt focal-pgdg main" > /etc/apt/sources.list.d/pgdg.list
RUN wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | apt-key add -
RUN apt-get update
RUN apt-get install -y postgresql-server-dev-14 postgresql-contrib-14

# ENV SSL_VERSION=1.0.2u
 
# RUN curl https://www.openssl.org/source/openssl-$SSL_VERSION.tar.gz -O && \
#     tar -xzf openssl-$SSL_VERSION.tar.gz && \
#     cd openssl-$SSL_VERSION && ./config && make depend && make install && \
#     cd .. && rm -rf openssl-$SSL_VERSION*
 
# ENV OPENSSL_LIB_DIR=/usr/local/ssl/lib \
#     OPENSSL_INCLUDE_DIR=/usr/local/ssl/include \
#     OPENSSL_STATIC=1

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup update stable \
   && rustup target add wasm32-unknown-unknown

RUN cargo install diesel_cli --no-default-features --features "postgres"

RUN git clone --branch v0.2.5 https://github.com/DA0-DA0/dao-contracts.git
RUN mkdir /app
ADD . /app
WORKDIR /app
RUN cargo build
CMD cargo run
