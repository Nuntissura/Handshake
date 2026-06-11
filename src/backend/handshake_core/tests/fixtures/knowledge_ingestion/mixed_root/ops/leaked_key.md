# Deploy notes (MT-096 secret fixture)

Every credential below is FAKE: the AWS key is the documented AWS example
value and the private key body is dummy text. The secret preflight must BLOCK
this file and never store its raw bytes.

aws_access_key_id = AKIAIOSFODNN7EXAMPLE

-----BEGIN RSA PRIVATE KEY-----
MIIfakefixturekeymaterial/notarealkey
-----END RSA PRIVATE KEY-----
