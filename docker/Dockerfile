FROM rust:1.73.0-bookworm
RUN useradd -d /code -c "The User" -M user
RUN apt update && apt install -y libasound2-dev libxcb-xfixes0-dev libxcb-shape0-dev
USER user
# RUN cd ~ &&  cargo install cargo-deb

# cargo install cargo-deb
# cargo deb
# rustup target add x86_64-pc-windows-gnu
# apt install mingw-w64
# cargo build --target x86_64-pc-windows-gnu
