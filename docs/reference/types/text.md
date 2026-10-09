# `text` Data Type

The `text` format stores UTF-8 text buffers.

## Architectural Storage

### Inverted Indexing
When a neuron is designated as `text`, Cluaizdb exposes it to full-text tokenizers. The text is parsed and indexed using term frequency mapping, allowing fast prefix, fuzzy, and BM25 matches.

## Use Cases
- Chat history logs, document search, and metadata tag indexing.
