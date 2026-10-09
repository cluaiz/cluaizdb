# 🐣 Chapter 1: Your First Mutating Database (Hello World)

Welcome to the absolute beginning. In this chapter, we will not use any heavy math or complex vector searches. We are just going to boot up the engine and inject a simple "Hello World" DNA.

By the end of this tutorial, you will understand how to:
1. Boot the cluaizdb Server.
2. Insert a basic Neuron (Data).
3. Query the Database.

---

## 1. Booting the Engine

Open your terminal, navigate to your workspace, and run:

```bash
cargo run -p cluaizdb-server
```

You will see logs indicating that the `LMDB environment` has opened and the server is listening on `0.0.0.0:7331`.

## 2. Injecting Your First Neuron

In a traditional database like SQL, you would `CREATE TABLE` and then `INSERT`. 
In Cluaizdb, we just **fire a Neuron**.

Open another terminal (or use Postman) and send a `POST` request to `/neuron`:

```bash
curl -X POST http://localhost:7331/neuron \
-H "Content-Type: application/json" \
-d '{
  "raw_payload": "Hello Cluaizdb! This is my first data.",
  "vector_data": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
  "model_creator_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "payload_type": "text",
  "dna": null
}'
```

**What just happened?**
The system absorbed your JSON payload, wrote it safely to the Write-Ahead Log (WAL) to prevent data loss, and then permanently mapped it into Zero-Copy RAM via LMDB. 
You will receive a response like `{"neuron_id":"019ea3d1...", "status":"created"}`.

## 3. Querying the Database (CDQL)

Now, how do we get that data back? We use **CDQL** (cluaizdb Query Language).

Send a `POST` request to `/query`:

```bash
curl -X POST http://localhost:7331/query \
-H "Content-Type: application/json" \
-d '{
  "cdql": "find *"
}'
```

You will get an array containing the Neuron you just inserted! 

---

### What's Next?
Right now, the `dna` field was `null`. This means the database acted exactly like a dumb storage engine (like Redis or standard MongoDB). 

In the next chapters, we will pass **Scripts (WASM/Rhai)** into that `dna` field so the database starts mutating incoming data on its own!
