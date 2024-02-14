#!/bin/sh

if [ -z "$MINT_LND_MACAROON_BASE64" ] || [ -z "$MINT_LND_TLS_CERT_BASE64" ]; then
    echo "Warning: MINT_LND_MACAROON_BASE64 and MINT_LND_TLS_CERT_BASE64 not set" >&2
    exec "$@"
    exit 0
fi

# Decode the base64 environment variables and write them to files
mkdir -p /tmp/lndconf
echo "$MINT_LND_MACAROON_BASE64" | base64 -d > /tmp/lndconf/admin.macaroon
if [ $? -ne 0 ]; then
    echo "MINT_LND_MACAROON_BASE64 is not valid base64"
    exit 1
fi

echo "$MINT_LND_TLS_CERT_BASE64" | base64 -d > /tmp/lndconf/tls.cert
if [ $? -ne 0 ]; then
    echo "MINT_LND_TLS_CERT_BASE64 is not valid base64"
    exit 1
fi

# Restrict permissions of the files
chmod 700 /tmp/lndconf
chmod 400 /tmp/lndconf/admin.macaroon
chmod 400 /tmp/lndconf/tls.cert

# Start your application
exec "$@"