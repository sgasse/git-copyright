
FROM rust:alpine as builder_rust

# Required in compiling some dependencies (at least proc-macro-error)
RUN apk add build-base

COPY ./ /repo
WORKDIR /repo

RUN cargo build --release

FROM alpine/git

COPY --from=builder_rust /repo/target/release/git_copyright /usr/bin/git_copyright

WORKDIR /mnt

ENTRYPOINT ["/usr/bin/git_copyright"]
