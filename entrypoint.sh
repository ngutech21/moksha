#!/bin/sh

# Decode the base64 environment variables and write them to files
mkdir -p /lndconf
echo "$LND_MACAROON_BASE64" | base64 -d > /lndconf/admin.macaroon
echo "$LND_TLS_CERT_BASE64" | base64 -d > /lndconf/tls.cert

# Start your application
exec "$@"