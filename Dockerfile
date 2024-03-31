FROM rust:1.76-bullseye as builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update
# RUN apt-get install -y extra-runtime-dependencies
RUN rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/dejavu-rs /usr/local/bin/dejavu-rs
EXPOSE 8000
CMD ["dejavu-rs"]