# Portal protocol notes

The client reproduces the requests made by the institute's local-auth page. These notes intentionally contain no real account, password, IP address, or session token.

## Common headers

Requests are HTTP `POST` with `application/x-www-form-urlencoded` bodies and include:

```text
HTTP_X_REQUESTED_WITH: xmlhttprequest
X-Requested-With: XMLHttpRequest
Origin: http://10.10.0.10:8008
Referer: http://10.10.0.10:8008/portal/local/index.html
```

No Cookie was present in the observed login, keepalive, or logout requests.

## Credential encoding

Username and password are encoded separately:

```text
Base64(AES-128-CBC-ZeroPadding(UTF8(value), fixed_key, random_ascii_iv)) + iv
```

The final 16 ASCII bytes are the IV. CryptoJS ZeroPadding does not append an additional block when the input is already block aligned.

## Login

```text
POST /portal.cgi
username=<encoded>&password=<encoded>&uplcyid=null&language=0&code=<code>&submit=submit
```

Responses:

- `0#message`: authentication failed.
- `1#url`: a browser-based second step is required.
- Success: eight `&`-separated fields containing encoded username, login time, IP, session secret, keepalive interval, timestamp, and account flags.

Before login, `/user_auth_verify.cgi` is queried. If its JSON response has `verify: 1`, the returned `code` is submitted directly; OCR is unnecessary.

## Keepalive

```text
POST /keepalive.cgi
secret=<session-secret>&submit=submit
```

`0#message` means offline. A successful response has eight fields; field 0 is the next interval in seconds. The observed value was 451 seconds, but clients must use the server value rather than hard-code it.

## Logout

```text
POST /logout.cgi
username=<plain-username>&secret=<session-secret>&language=0&submit=submit
```

Any response not beginning with `0#` is treated as success, matching the browser implementation.

