# syntax=docker/dockerfile:1.6

ARG UBUNTU_VERSION=24.04
ARG GO_VERSION=1.24

############################
# Sunspot Builder
############################

FROM golang:${GO_VERSION}-bookworm AS sunspot-build

ARG SUNSPOT_COMMIT=dfcbc19df024dcdc2199f8bbe1c485d19a6c5617                                  

RUN git clone https://github.com/reilabs/sunspot.git /sunspot
RUN git -C /sunspot checkout ${SUNSPOT_COMMIT}
WORKDIR /sunspot/go

RUN go build -o sunspot .

############################
# Runtime stage
############################
FROM ubuntu:${UBUNTU_VERSION} AS runtime

ARG DEBIAN_FRONTEND=noninteractive
ARG NOIR_VERSION=1.0.0-beta.13      

ENV HOME="/root"
ENV NARGO_HOME="${HOME}/.nargo"


RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git \
  && rm -rf /var/lib/apt/lists/*

# Install Noirlang
RUN mkdir -p "${NARGO_HOME}/bin" \
  && curl -# -L https://github.com/noir-lang/noirup/releases/latest/download/noirup -o noirup \
  && chmod +x noirup \
  && ./noirup --version ${NOIR_VERSION} \
  && NARGO_BIN="$(find /root -maxdepth 4 -type f -path '*/bin/nargo' | head -n 1)" \
  && ln -s "${NARGO_BIN}" /usr/local/bin/nargo

ENV PATH="${NARGO_HOME}/bin:/sunspot:${PATH}"

# Copy sunspot
COPY --from=sunspot-build /sunspot/go/sunspot /usr/local/bin/sunspot

# Copy the gnark circuit and trusted setup
COPY circuits/condenser /circuits/condenser

COPY docker/prove.sh /usr/local/bin/prove.sh
RUN chmod +x /usr/local/bin/prove.sh

# Build the circuit so the dependencies are cached
RUN cd /circuits/condenser && nargo build

ENTRYPOINT ["/usr/local/bin/prove.sh"]
