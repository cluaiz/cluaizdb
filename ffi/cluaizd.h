/**
 * cluaizdb.h — CLUAIZD C FFI Header
 *
 * Cluaizdb Nervous System Database — C-Compatible Interface
 * Copyright (c) 2024 Cluaizdb. All rights reserved.
 *
 * BSL-1.1 License — Free for personal use and research.
 * Commercial cloud service deployment requires a separate license.
 *
 * ============================================================
 * USAGE EXAMPLE
 * ============================================================
 *
 *   #include "cluaizdb.h"
 *   #include <stdio.h>
 *   #include <string.h>
 *
 *   int main() {
 *       // Open database (4GB max size)
 *       CluaizdbHandle* db = cluaizdb_open("./data/mydb", 4096);
 *       if (!db) { fprintf(stderr, "Failed to open CLUAIZD\n"); return 1; }
 *
 *       // Write a neuron
 *       const char* payload = "{\"name\":\"Aryan\",\"age\":25}";
 *       char* id = cluaizdb_write(db, (const uint8_t*)payload, strlen(payload), "text");
 *       if (id) {
 *           printf("Written neuron: %s\n", id);
 *           cluaizdb_free_string(id);
 *       }
 *
 *       // Query neurons
 *       char* results = cluaizdb_query(db, "Aryan");
 *       if (results) {
 *           printf("Results: %s\n", results);
 *           cluaizdb_free_string(results);
 *       }
 *
 *       // Close
 *       cluaizdb_close(db);
 *       return 0;
 *   }
 *
 * ============================================================
 */

#ifndef CLUAIZD_H
#define CLUAIZD_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Opaque handle to an open CLUAIZD instance.
 * Never access the internal fields directly.
 */
typedef struct CluaizdbHandle CluaizdbHandle;

/**
 * Open a CLUAIZD at the given filesystem path.
 *
 * @param path          UTF-8 encoded path to the database directory.
 *                      Will be created if it doesn't exist.
 * @param map_size_mb   Maximum database size in megabytes.
 *                      (e.g. 4096 = 4 GB, 131072 = 128 GB)
 *
 * @return A valid handle on success, or NULL on failure.
 *         Must be freed with cluaizdb_close().
 */
CluaizdbHandle* cluaizdb_open(const char* path, unsigned long map_size_mb);

/**
 * Write a raw payload into CLUAIZD and get back a unique Neuron ID.
 *
 * @param handle        A valid handle from cluaizdb_open().
 * @param payload       Pointer to the raw byte data.
 * @param payload_len   Length of the payload in bytes.
 * @param payload_type  Type string: "text", "audio", "video", "code",
 *                      "voltage_stream", or "binary".
 *
 * @return A heap-allocated null-terminated UUID string (the Neuron ID).
 *         MUST be freed with cluaizdb_free_string(). Returns NULL on failure.
 */
char* cluaizdb_write(
    CluaizdbHandle* handle,
    const uint8_t* payload,
    size_t payload_len,
    const char* payload_type
);

/**
 * Read a neuron's raw payload from CLUAIZD by its UUID.
 *
 * @param handle        A valid handle from cluaizdb_open().
 * @param neuron_id     Null-terminated UUID string of the neuron to read.
 * @param out_len       Pointer to an unsigned long that receives the payload length.
 *
 * @return A heap-allocated byte buffer containing the raw payload.
 *         MUST be freed with cluaizdb_free_bytes(out_len). Returns NULL if not found.
 */
uint8_t* cluaizdb_read(
    CluaizdbHandle* handle,
    const char* neuron_id,
    unsigned long* out_len
);

/**
 * Query CLUAIZD using a CDQL string or keyword.
 * Returns a JSON array of matching Neuron IDs.
 *
 * @param handle    A valid handle from cluaizdb_open().
 * @param CDQL      A null-terminated query string.
 *                  Example: `find *(name: "Aryan")` or just `Aryan`
 *
 * @return A heap-allocated null-terminated JSON string: ["id1", "id2", ...]
 *         MUST be freed with cluaizdb_free_string(). Returns NULL on failure.
 */
char* cluaizdb_query(CluaizdbHandle* handle, const char* CDQL);

/**
 * Close a CLUAIZD handle and release all associated memory.
 *
 * @param handle    A valid handle from cluaizdb_open(). NULL-safe.
 */
void cluaizdb_close(CluaizdbHandle* handle);

/**
 * Free a string returned by cluaizdb_write() or cluaizdb_query().
 *
 * @param ptr   Pointer returned by cluaizdb_write()/cluaizdb_query(). NULL-safe.
 */
void cluaizdb_free_string(char* ptr);

/**
 * Free a byte buffer returned by cluaizdb_read().
 *
 * @param ptr   Pointer returned by cluaizdb_read().
 * @param len   The length value written to out_len by cluaizdb_read().
 */
void cluaizdb_free_bytes(uint8_t* ptr, unsigned long len);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* CLUAIZD_H */
