#!/bin/sh

if [ -z "$LND_MACAROON_BASE64" ] || [ -z "$LND_TLS_CERT_BASE64" ]; then
    echo "Warning: LND_MACAROON_BASE64 and LND_TLS_CERT_BASE64 not set" >&2
    exec "$@"
    exit 0
fi

# Decode the base64 environment variables and write them to files
mkdir -p /tmp/lndconf
echo "$LND_MACAROON_BASE64" | base64 -d > /tmp/lndconf/admin.macaroon
if [ $? -ne 0 ]; then
    echo "LND_MACAROON_BASE64 is not valid base64"
    exit 1
fi

echo "$LND_TLS_CERT_BASE64" | base64 -d > /tmp/lndconf/tls.cert
if [ $? -ne 0 ]; then
    echo "LND_TLS_CERT_BASE64 is not valid base64"
    exit 1
fi

# Restrict permissions of the files
chmod 700 /tmp/lndconf
chmod 400 /tmp/lndconf/admin.macaroon
chmod 400 /tmp/lndconf/tls.cert

#chown -R 1000:1000 /tmp/lndconf

# Start your application
exec "$@"