#!/bin/sh

# Decode the base64 environment variables and write them to files
mkdir -p /lndconf
echo "$LND_MACAROON_BASE64" | base64 -d > /lndconf/admin.macaroon
if [ $? -ne 0 ]; then
    echo "LND_MACAROON_BASE64 is not valid base64"
    exit 1
fi

echo "$LND_TLS_CERT_BASE64" | base64 -d > /lndconf/tls.cert
if [ $? -ne 0 ]; then
    echo "LND_TLS_CERT_BASE64 is not valid base64"
    exit 1
fi

# Restrict permissions of the files
chmod 700 /lndconf
chmod 400 /lndconf/admin.macaroon
chmod 400 /lndconf/tls.cert

# Start your application
exec "$@"