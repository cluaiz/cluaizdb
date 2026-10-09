#ifndef CLUAIZDB_H
#define CLUAIZDB_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/* Status / Return Codes */
#define CLUAIZDB_OK                      0
#define CLUAIZDB_ERR_NULL_HANDLE        -1
#define CLUAIZDB_ERR_NOT_FOUND          -2
#define CLUAIZDB_ERR_BUFFER_TOO_SMALL   -3
#define CLUAIZDB_ERR_STORAGE            -4
#define CLUAIZDB_ERR_PANIC              -5

/**
 * Opens or initializes a physical CluaizDB Substrate environment.
 *
 * @param path Null-terminated UTF-8 filesystem path.
 * @param map_size Memory-mapped maximum virtual address size in bytes.
 * @return Opaque pointer to the Substrate instance, or NULL on failure.
 */
void* cluaizdb_open(const char* path, size_t map_size);

/**
 * Closes and safely releases an open Substrate instance.
 *
 * @param handle Opaque pointer returned by cluaizdb_open.
 */
void cluaizdb_close(void* handle);

/**
 * Puts a raw byte record into the primary records table.
 *
 * @param handle Opaque pointer to Substrate.
 * @param col_id Collection identifier (u32).
 * @param id_hi Most significant 64 bits of the 128-bit record ID.
 * @param id_lo Least significant 64 bits of the 128-bit record ID.
 * @param payload Pointer to raw byte payload.
 * @param payload_len Length of raw payload in bytes.
 * @return CLUAIZDB_OK (0) on success, negative error code otherwise.
 */
int32_t cluaizdb_put(
    void* handle,
    uint32_t col_id,
    uint64_t id_hi,
    uint64_t id_lo,
    const uint8_t* payload,
    size_t payload_len
);

/**
 * Gets a raw byte record by ID from the primary records table (< 5 µs recall).
 *
 * @param handle Opaque pointer to Substrate.
 * @param col_id Collection identifier.
 * @param id_hi Most significant 64 bits of record ID.
 * @param id_lo Least significant 64 bits of record ID.
 * @param out_buf Caller-allocated buffer to receive the payload.
 * @param max_buf_len Maximum capacity of out_buf in bytes.
 * @param out_len Pointer to receive actual bytes copied.
 * @return CLUAIZDB_OK (0) on success, CLUAIZDB_ERR_NOT_FOUND (-2) if missing.
 */
int32_t cluaizdb_get(
    void* handle,
    uint32_t col_id,
    uint64_t id_hi,
    uint64_t id_lo,
    uint8_t* out_buf,
    size_t max_buf_len,
    size_t* out_len
);

/**
 * Deletes a record by ID.
 *
 * @param handle Opaque pointer to Substrate.
 * @param col_id Collection identifier.
 * @param id_hi Most significant 64 bits of record ID.
 * @param id_lo Least significant 64 bits of record ID.
 * @return CLUAIZDB_OK (0) if deleted, CLUAIZDB_ERR_NOT_FOUND (-2) if did not exist.
 */
int32_t cluaizdb_delete(
    void* handle,
    uint32_t col_id,
    uint64_t id_hi,
    uint64_t id_lo
);

/**
 * Forces physical disk sync barrier on the environment (force_sync).
 *
 * @param handle Opaque pointer to Substrate.
 * @return CLUAIZDB_OK (0) on success.
 */
int32_t cluaizdb_sync(void* handle);

#ifdef __cplusplus
}
#endif

#endif /* CLUAIZDB_H */
