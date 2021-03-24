#!/usr/bin/env bash

set -e

if [[ $# -eq 0 ]] ; then
    echo 'No version provided'
    exit 0
fi

echo "Building and publishing version $1"

docker build -t noxious:$1 .
docker image tag noxious:$1 oguzbilgener/noxious:$1
docker image tag noxious:$1 oguzbilgener/noxious:latest
docker push "oguzbilgener/noxious:$1"