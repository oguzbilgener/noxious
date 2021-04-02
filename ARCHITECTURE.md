# Architecture

Noxious server consists of a core crate, and a server crate.

The `core` crate contains the serializable data types, as well as the proxy runner, toxics and some state management. The `server` crate contains the REST API server and the `Store` which manages proxy and toxic CRUD operations.

The `client` library uses the types from `core` offers an async client to the Toxiproxy API interface.

### Link

For every client connection, an upstream and a downstream `Link` are created. Each link owns a read and write handle, as well as the toxics for its direction. When a link is established, the read and write handles are connected via toxics.

### Toxic

In Noxious, most toxics simply take an input `Stream` of `Bytes`, and write data to a `Sink` of `Bytes`. Data passes through a chain of toxics and each toxic can manipulate the data passed, i.e. add delay, split packets into smaller packets etc. Some toxics like `SlowClose` and `LimitData` need more information about the connection, so they also take a `Stop` signal.

### Toxic Runner

Noxious has a few key differences from Toxiproxy in how it adds and executes toxics on proxy connections:

1. It does not insert a `Noop` toxic as the first toxic in the chain.
2. When the toxics are updated for a proxy, it re-creates links with new toxic chains instead of mutating the existing toxic chain, without closing the proxy connection.
3. When a proxy is updated, it drops the old proxy, causing old connection to disconnect. This is practically the same behavior as Toxicproxy, as if you update the listen address or upstream address, you must close the proxy connections.