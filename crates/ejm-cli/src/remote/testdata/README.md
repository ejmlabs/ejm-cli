# Untrusted relay TLS fixture

The planned native layer's certificate and PKCS8 key are generated test material with no production identity.
The self-signed ECDSA P-256 certificate covers only `127.0.0.1`, has server-auth use,
and is valid from January 1, 2020 through January 1, 2120. It is never installed in
the OS trust store. The account TLS regression trusts it only for one local probe
that proves the fixture can serve HTTP; the production account client must reject it.

Generated with OpenSSL 4.0.3. That layer includes them only in test builds; this docs prerequisite includes no fixture binaries.
