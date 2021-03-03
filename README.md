# noxious

[![Crates.io](https://img.shields.io/crates/v/noxious)](https://crates.io/crates/noxious)

A Rust port of [Toxiproxy][toxiproxy] server, which is a TCP proxy to simulate network and system conditions for chaos and resiliency testing.

Noxious is fully compatible with Toxiproxy with the same REST API, so you can use the Toxiproxy CLI and all the existing [client libraries][clients] for Toxiproxy with noxious.

A Rust client library as well as a CLI tool is also in active development.


[toxiproxy]: https://github.com/Shopify/toxiproxy
[clients]: https://github.com/Shopify/toxiproxy#clients

### Installation

Noxious server Docker image will be available on Docker hub soon.


### Usage

Run `./noxious-server --help` to see the list of available commands.

By default the API server listens on **127.0.0.1:8474**.

#### Populating Proxies

You can run noxious-server with `--config` to provide the path to a JSON file that contains a list of initial proxies. Alternatively, you can use the CLI or the REST API to create proxies:

```sh
toxiproxy-cli create test_redis -l localhost:26379 -u localhost:6379
```

#### Adding Toxics

You can add toxics using the client libraries, or via the CLI:

```sh
toxiproxy-cli toxic add test_redis -t latency -a latency=1000
```

See the [Toxiproxy README][toxics_docs] for the full documentation of toxics.

[toxics_docs]: https://github.com/Shopify/toxiproxy#toxics