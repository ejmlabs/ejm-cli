# Untrusted relay TLS fixture

The certificate and PKCS8 key are generated test material with no production identity.
The self-signed ECDSA P-256 certificate covers only `127.0.0.1`, has server-auth use,
and is valid from January 1, 2020 through January 1, 2120. It is never installed in
the OS trust store. The TLS regression checks rejection before HTTP bearer/OAuth
data or a WSS ticket reaches the peer. A separate local probe trusts only this
certificate to prove the fixture can serve HTTP.

Generated with OpenSSL 4.0.3. The files are included only in test builds.
