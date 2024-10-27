FROM rust:1.76.0 AS builder

LABEL org.opencontainers.image.authors="charlie@wharflabs.com"
LABEL org.opencontainers.image.description="Rust runtime for Jupyter notebooks, with a python3 kernel"
LABEL org.opencontainers.image.source="https://www.github.com/runtimed/runtimed"

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release --bin runtimed

FROM python:3.12

RUN pip3 install ipykernel && python3 -m ipykernel install
COPY --from=builder /usr/src/app/target/release/runtimed /usr/local/bin/runtimed

EXPOSE 12397
ENTRYPOINT [ "runtimed" ]
