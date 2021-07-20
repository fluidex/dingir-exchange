#!/bin/bash
set -u

if [ $# -ne 2 ]
then
    echo "Usage: $0 docker-registry image-tag"
    exit 1
fi

DOCKER_IMAGE_NAME='dingir-exchange-matchengine'
DOCKER_TARGET="$1/$DOCKER_IMAGE_NAME:$2"

function run() {
    install_cross
    build_release
    docker_build
    help_info
}

function install_cross() {
    echo 'install cross - https://github.com/rust-embedded/cross'
    cargo install cross
}

function build_release() {
    echo 'build a release for target x86_64-unknown-linux-gnu'
    RUSTFLAGS="-C link-arg=-static -C target-feature=+crt-static" cross build --bin matchengine --target x86_64-unknown-linux-gnu --release
}

function docker_build() {
    echo "docker build a image $DOCKER_TARGET"
    docker build -t $DOCKER_TARGET -f release/Dockerfile .
}

function help_info() {
    echo "Push to Docker Registry: docker push $DOCKER_TARGET"
    echo "Run a new Docker Container: docker run $DOCKER_TARGET"
}

run
