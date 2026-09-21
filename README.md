Tarbaby
=======

The slowloris attack in reverse, to waste the time and bandwidth vulnerability scanners.

This is a simple command-line web server that ... will never finish answering an HTTP request :-)

It supports HTTP and HTTPS.


Wut?
----

A source of perverse entertainment is to simply rent a cloud server, point dns at it, start
a web server, and watch the log file as it is bombarded day and night with requests for things
like `.git/config`, `.env`, and the endless parade of historical PHP vulnerabilities - a veritable
history of software vulnerabilities and developer mistakes.

The [slowloris attack](https://www.cloudflare.com/learning/ddos/ddos-attack-tools/slowloris/) is a
flavor of denial-of-service attack, using an http client deliberately misdesigned to send requests
that never reach completion. The observation it exploits is that *HTTP **is** a stateful protocol
until the request headers have been completely sent*.

Most web servers defend against this kind of attack these days.  But not all clients do (real web browsers
largely do), including those used by exploit scanners - I've kept one tied up for nearly a week receiving
nonsense headers, one per second.  And while you're keeping them tied up, they can't use that port to
scan someone who actually is vulnerable.

So, `tarbaby` just provides a little bit of schedenfreude.  Is it good for something?  Who knows.  But
it felt good to write it.


How It Works
------------

When a request arrives, it will send a normal HTTP 1.1 response line.  Followed by a `Transfer-Encoding: chunked`
header, and the required `Date` header.  After that, it will send one random-string header per second (or whatever
delay you set) until the end of time, or until client gives up and breaks the connection.  E.g.


```sh
curl -i http://localhost:6666/hey/there    

HTTP/1.1 200 OK
Transfer-Encoding: chunked
Date: Sun, 20 Sep 2026 21:49:13 GMT
PiZAH-LIavMS: srtplvRVnusbDFRstDmyA
gRTewYj-wglZIAs: lLmIPaGWnbsgMRrkF
vXlM-gFFrVP: hLjbRkpPsRL
tRK-vbul: odkDprcJd
# ... and so forth
```

Internally, it uses async I/O, so running it is cheap, each connection does *not* cost a system-level
thread, and the only limit on concurrent connections is your operating system's.


Build and Run
-------------

Just build it as a normal rust application:

```sh
cargo build --release
```

copy the binary built into `target/release/tarbaby` somewhere, and run it.  Set the `RUST_LOG` environment
variable to emit logging, e.g. `RUST_LOG=info` (other options are `error`, `warn`, `debug` and `trace`), e.g.

```sh
RUST_LOG=info tarbaby serve --port 6666
```

Passing `-h` or `--help` will explain the command line options (port, interface, delay between headers, etc.).

It will periodically log the number of headers it has sent, connections that have been opened and closed, and
similar.

Run it somewhere on your server - *do **not** put it behind a proxy server*, as they all try to gather up all
the headers before forwarding a response, which defeats the purpose.

Then set up your proxy server to *redirect* requests to it, using a regular expression like this one for NginX:

```
location        ~* (?:\.env|\.azure|\.git|graphql|ecp|\.env.bak|\.env.old|functions|\.env.dev|\.env.staging|\.aws|wp-config\.php|xmlrpc\.php|autoload\.php|config\.json|elFinder|backup.*\.sql)
```

That way, if the scanner follows the redirect, `tarbaby` can do whatever it wants with them from there out.


#### Feature Flags

The building with `-F multithread` enables the multithreaded tokio runtime, rather than the default, node-js style
single-threaded one.  Bear in mind, requests in this server spend nearly *all* of their time idle.  You probably
don't need it.


HTTPS Support
-------------

`tarbaby` can support HTTPS - simply pass `-c`/`--certificate` and the path to a certificate `.pem` file, 
and `-k`/`--key` and a path to the `.pem` private key file.  Note that the certificate file should be the full
certificate chain.  For example, if you were using [letsencrypt](https://letsencrypt.org/), you would want
to use the `fullchain.pem` for the certificate and the `privkey.pem` files from the `live/` directory
(depending on the OS, under `/etc/letsencrypt` or `/var/lib/letsencrypt`).

Running in both HTTP and HTTPs modes is not supported, but the application is quite lightweight and running
two instances on different ports is trivial.

For a demo of https support, run the `demo.sh` script in the checkout root.

License
-------

Licensed under the [GNU General Public License 2.0](https://spdx.org/licenses/GPL-2.0-only.html).
