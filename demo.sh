#!/bin/bash

set -e

ROOT=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

rm -Rf "${ROOT}/demo"
mkdir -p "${ROOT}/demo"

cd "${ROOT}/demo"

echo "Generating self-signed cert. This will fail if openssl is not installed."

openssl req -x509 -newkey rsa:4048 -nodes -keyout key.pem -out cert.pem -days 365 -subj "/C=US/ST=State/L=Locality/O=Organization/OU=OrganizationalUnit/CN=localhost" &> /dev/null

cd "${ROOT}"

echo
echo "*************************************************************"
echo "*                                                           *"
echo "*   Once the server has started, try something like:        *"
echo "*   curl -i --insecure https://localhost:6666/hello/world   *"
echo "*                                                           *"
echo "*************************************************************"
echo

RUST_LOG=debug cargo run --release -- serve -c "${ROOT}/demo/cert.pem" -k "${ROOT}/demo/key.pem"
