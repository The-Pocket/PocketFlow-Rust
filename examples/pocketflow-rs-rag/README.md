# PocketFlow RAG Example

## Overview

This example demonstrates how to use PocketFlow to build a Retrieval-Augmented Generation (RAG) pipeline. The implementation consists of two main components: an offline pipeline for document processing and indexing, and an online pipeline for question answering.

### Offline Pipeline

The offline pipeline processes and indexes documents for later retrieval. It consists of the following nodes:

- `FileLoaderNode`: Loads documents from local files or URLs, supporting various formats including PDF, text, and web pages.
- `ChunkDocumentsNode`: Splits documents into smaller chunks using configurable chunk size and overlap, with support for different chunking strategies.
- `EmbedDocumentsNode`: Converts document chunks into vector embeddings using OpenAI's embedding models.
- `CreateIndexNode`: Stores the embedded chunks in a Qdrant vector database with configurable distance metrics.

### Online Pipeline

The online pipeline handles real-time question answering using the indexed documents. It includes:

- `QueryRewriteNode`: Enhances the user's query using LLM to improve retrieval quality.
- `EmbedQueryNode`: Converts the rewritten query into a vector embedding.
- `RetrieveDocumentNode`: Retrieves the most relevant document chunks from the vector database.
- `GenerateAnswerNode`: Generates a comprehensive answer based on the retrieved context and the original query.

The pipeline supports various configuration options including:

- Customizable embedding models and dimensions
- Configurable chunk sizes and overlap
- Adjustable number of retrieved documents
- Different chat modes for answer generation
- Flexible vector database settings

## Workflow Diagram

```mermaid
graph TB
    subgraph Offline["Offline Pipeline"]
        direction LR
        FL[FileLoaderNode] --> CD[ChunkDocumentsNode]
        CD --> ED[EmbedDocumentsNode]
        ED --> CI[CreateIndexNode]
    end

    subgraph Online["Online Pipeline"]
        direction LR
        QR[QueryRewriteNode] --> EQ[EmbedQueryNode]
        EQ --> RD[RetrieveDocumentNode]
        RD --> GA[GenerateAnswerNode]
    end

    style Offline fill:#f9f,stroke:#333,stroke-width:2px
    style Online fill:#bbf,stroke:#333,stroke-width:2px
```

## Example Usage

### run offline pipeline

```bash
cargo run -- offline --db-url <qdrant-db-url> --collection <collection-name> --api-key <openai-api-key> --qdrant-api-key <qdrant-api-key> --endpoint <openai-endpoint> --chunk-size <chunk-size> --overlap <overlap> --model <embedding-model> --dimension <dimension> https://www.usenix.org/system/files/fast23-li-qiang_more.pdf https://www.usenix.org/system/files/fast23-li-qiang.pdf
```

### run online pipeline

```bash
cargo run -- online --db-url <qdrant-db-url> --collection <collection-name> --api-key <openai-api-key> --qdrant-api-key <qdrant-api-key> --endpoint <openai-endpoint> --embedding-model <embedding-model> --chat-mode <chat-mode> --dimension <dimension> --k <k> "Introduce Alibaba Cloud's Pangu distributed file system"
```

### Output

```markdown
Alibaba Cloud's Pangu is a large-scale, distributed storage system that has been in development and deployment since 2009. It serves as a unified storage platform for Alibaba Group and Alibaba Cloud, providing scalable, high-performance, and reliable storage services to support core businesses such as Taobao, Tmall, AntFin, and Alimama. A variety of cloud services, including Elastic Block Storage (EBS), Object Storage Service (OSS), Network-Attached Storage (NAS), PolarDB, and MaxCompute, are built on top of Pangu. Over more than a decade, Pangu has grown into a global storage system managing exabytes of data and trillions of files.

### Evolution of Pangu

Pangu's evolution can be divided into two main phases:

1. **Pangu 1.0 (2009-2015)**: This version was designed on an infrastructure composed of servers with commodity CPUs and hard disk drives (HDDs), which have millisecond-level I/O latency, and Gbps-level datacenter networks. Pangu 1.0 featured a distributed kernel-space file system based on Linux Ext4 and kernel-space TCP, gradually adding support for multiple file types (e.g., TempFile, LogFile, and random access files) as required by different storage services. During this period, the primary focus was on providing large volumes of storage space rather than high performance.

2. **Pangu 2.0 (Since 2015)**: In response to the emergence of new hardware technologies, particularly solid-state drives (SSDs) and remote direct memory access (RDMA), Pangu 2.0 was developed to provide high-performance storage services with a 100µs-level I/O latency. Key innovations include:
   - **Embracing SSD and RDMA**: To leverage the low latency of SSDs and RDMA, Pangu 2.0 introduced a series of new designs in its file system and developed a user-space storage operating system.
   - **High Throughput and IOPS**: Pangu 2.0 aims to achieve high throughput and IOPS, with an effective throughput on storage servers approaching their capacity.
   - **Unified High-Performance Support**: The system provides unified high-performance support to all services running on top of it, such as online search, data streaming analytics, EBS, OSS, and databases.

### Design Goals of Pangu 2.0

- **Low Latency**: Pangu 2.0 targets an average 100µs-level I/O latency in a computation-storage disaggregated architecture, even under dynamic environments like network traffic jitters and server failures.
- **High Throughput**: The system aims to reach an effective throughput on storage servers that approaches their capacity.
- **Unified High-Performance Support**: Pangu 2.0 provides unified high-performance support to all services, ensuring that all applications benefit from the advancements in hardware and software.

### Related Work

Pangu is part of a broader ecosystem of distributed storage systems, both open-source (e.g., HDFS and Ceph) and proprietary (e.g., GFS, Tectonic, and AWS). Alibaba has shared its experiences in various aspects of Pangu, including the large-scale deployment of RDMA, key-value engines for scale-out cloud storage, co-design of network and storage software stacks for EBS, and key designs of the namespace metadata service.

For more detailed information, you can refer to the following sources:

- [FAST '23 Paper: "Fisc: A Lightweight Client for Large-Scale Distributed File Systems"](https://www.usenix.org/system/files/fast23-li-qiang.pdf)_more.pdf)

These documents provide in-depth insights into the design, implementation, and operational experiences of Pangu.
```
