# noxious

![Docker Image Version (latest by date)](https://img.shields.io/docker/v/oguzbilgener/noxious)
[![Unit Tests](https://github.com/oguzbilgener/noxious/actions/workflows/unit_tests.yml/badge.svg)](https://github.com/oguzbilgener/noxious/actions/workflows/unit_tests.yml)
[![Coverage Status](https://coveralls.io/repos/github/oguzbilgener/noxious/badge.svg?branch=main)](https://coveralls.io/github/oguzbilgener/noxious?branch=main)
![Crates.io](https://img.shields.io/crates/l/noxious)
[![semantic-release](https://img.shields.io/badge/semantic--release-enabled-brightgreen?logo=semantic-release)](https://github.com/semantic-release/semantic-release)

A Rust port of [Toxiproxy] server, which is a TCP proxy to simulate network and system conditions for chaos and resiliency testing.

Noxious is fully compatible with Toxiproxy with the same REST API, so you can use the Toxiproxy CLI and all the existing [client libraries][clients] for Toxiproxy with noxious.

An async Rust client library called [noxious-client] is also available to interact with Noxious or Toxiproxy.

Also check out [ARCHITECTURE.md] for implementation details.

[Toxiproxy]: https://github.com/Shopify/toxiproxy
[clients]: https://github.com/Shopify/toxiproxy#clients
[noxious-client]: https://docs.rs/noxious-client
[ARCHITECTURE.md]: [https://github.com/oguzbilgener/noxious/blob/main/ARCHITECTURE.md]

### Quick Start

Noxious server is available on [Docker Hub] and [GitHub Packages] for AMD64 and ARM. You can also find the executables for linux/amd64 in the [Releases] page.

Alternatively, you can build Noxious from source with [cargo]. Run the `cargo build --release` command and the executable will be available at `./target/release/noxious-server`.

You can run `noxious-server --help` to get the list of arguments. By default the API server listens on port **8474**. This can be changed by providing the `--port` command line argument. You can provide a JSON config file that declares an array of proxies to be created on startup with the `--config  ./path/to/file.json` argument.

For an extensive guide on how to use the Toxiproxy clients, please visit the [Toxiproxy] GitHub repository.

[Docker Hub]: https://hub.docker.com/repository/docker/oguzbilgener/noxious
[GitHub Packages]: https://github.com/users/oguzbilgener/packages/container/package/noxious
[Releases]: https://github.com/oguzbilgener/noxious/releases
[cargo]: https://doc.rust-lang.org/book/ch01-01-installation.html#installation

#### With Docker

 When running in Docker, you will need to make sure that Noxious can reach the services that you are testing, and you can reach the ports that Noxious exposes for these services. You can use docker-compose, host networking or a bridge network, as described below.

Suppose you have a web service running in the `myserver` container, connected to network `my-net`, listening on port `8000`:

```sh
docker network create -d bridge my-net
docker run --name myserver --rm -p 8000:8000 --network=my-net myimage:latest
```

You can start Noxious with a command like:


```sh
docker run --name noxious \
           --rm \
           -p 8474:8474 \
           -p 8001:8001 \
           --network=my-net \
           oguzbilgener/noxious
```

You can create the proxy by using one of the [clients] or the toxiproxy-cli, or by using cURL:

```sh
curl --request POST \
  --url http://localhost:8474/proxies \
  --header 'Content-Type: application/json' \
  --data '{
    "name": "myserver",
    "listen": "0.0.0.0:8001",
    "upstream": "myserver:8000",
    "enabled": true
}'
```

Now you should be able to access your server via Noxious at `http://localhost:8001`, or at `http://noxious:8001` from another container within the same Docker network.

You can add a latency toxic to simulate a bad network condition:

```sh
curl --request POST \
  --url http://localhost:8474/proxies/myserver/toxics \
  --header 'Content-Type: application/json' \
  --data '{
	"name": "myserver_latency",
	"type":"latency",
	"toxicity": 1,
	"direction": "upstream",
	"attributes": {
		"latency": 200,
		"jitter": 50
	}
}'
```


#### Populating Proxies

In addition to the initial JSON config, you can use the CLI or the REST API clients to create proxies:

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

### License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.