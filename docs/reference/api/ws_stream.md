# `ws:// /stream` WebSocket API

While HTTP is excellent for ingestion (`POST /neuron`), it is highly inefficient for returning millions of records due to the Request-Response buffering limits. Cluaizdb utilizes a persistent WebSocket connection for all CDQL read queries (`find`), enabling asynchronous, zero-copy data streaming.

## Architectural Execution

### 1. Persistent Socket Pools
When a client application connects to `ws://.../stream`, the Engine allocates a dedicated persistent socket. This socket remains open, eliminating the latency penalty of the TCP 3-Way Handshake and TLS negotiation for subsequent queries.

### 2. Backpressure and Memory Map Streaming
When a `find *` query is executed over the WebSocket, the Engine does not build a massive JSON Array in RAM. Instead, it reads a single record from the LMDB memory map, streams the binary frame down the WebSocket, and then reads the next. If the client's network connection slows down, the OS TCP buffer fills up. The Tokio runtime senses this **Backpressure** and pauses the memory map iteration, guaranteeing that the database server will never crash from Out-Of-Memory (OOM) errors, regardless of query size.

## API Specification

**Endpoint:** `ws://<HOST>:7331/stream`

### Sending a CDQL Query

To execute a query, transmit a stringified JSON object containing the `cdql` command through the socket.

```json
{
  "cdql": "find json where role == 'admin' limit 100",
  "query_vector": null
}
```

### Receiving the Stream

The server will respond with multiple WebSocket frames. The final frame will contain an End-Of-Stream (EOS) signal.

```javascript
const socket = new WebSocket("ws://127.0.0.1:7331/stream");

socket.onmessage = (event) => {
    let response = JSON.parse(event.data);
    
    if (response.status === "EOS") {
        console.log("Stream Complete.");
    } else {
        console.log("Received Record:", response.payload);
    }
};
```
