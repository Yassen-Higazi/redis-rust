[![progress-banner](https://backend.codecrafters.io/progress/redis/b1b50667-9b16-4f22-9717-ac00f3426a31)](https://app.codecrafters.io/users/codecrafters-bot?r=2qF)

# redis-rust

A Redis clone built in Rust, inspired by the [Build Your Own Redis](https://app.codecrafters.io/courses/redis/overview) challenge by CodeCrafters.

This project implements key Redis functionalities, including RESP parsing, in-memory data storage, key expiration, RDB file loading, and multi-client handling using asynchronous I/O.

---

<!--toc:start-->

- [🚀 Features](#🚀-features)
- [🛠 Installation](#🛠-installation)
  - [Prerequisites](#prerequisites)
  - [Clone the Repository](#clone-the-repository)
  - [Build the Project](#build-the-project)
  - [▶️ Running the Server](#️-running-the-server)
    - [Start Normally](#start-normally)
    - [Load Data from an RDB File](#load-data-from-an-rdb-file)
  - [💬 Connecting & Using redis-cli](#💬-connecting-using-redis-cli)
- [✅ Examples of Supported Commands](#examples-of-supported-commands)
  - [🗝️ KEYS (is loaded from RDB)](#🗝️-keys-is-loaded-from-rdb)
  - [📝 SET](#📝-set)
  - [📖 GET](#📖-get)
  - [⚙️ CONFIG](#️-config)
- [📄 License](#📄-license)
<!--toc:end-->

---

## 🚀 Features

- RESP (Redis Serialization Protocol) serialization & deserializer
- TCP server handling multiple concurrent clients
- Asynchronous I/O using Tokio
- Basic Redis command support: `PING`, `SET`, `GET`, `CONFIG`, `KEYS`
- Passive key expiration
- RDB file parsing and in-memory data loading

---

## 🛠 Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)

### Clone the Repository

```bash
git clone https://github.com/Yassen-Higazi/redis-rust.git
cd redis-rust
```

### Build the Project

```bash
cargo build
```

---

### ▶️ Running the Server

#### Start Normally

```bash
cargo run
```

#### Load Data from an RDB File

```bash
cargo run -- -d ./data
```

This will parse the dump.rdb file in the ./data directory and populate the in-memory store on startup.

### 💬 Connecting & Using redis-cli

```bash
redis-cli ping
PONG
```

---

## ✅ Examples of Supported Commands

### 🗝️ KEYS (is loaded from RDB)

```bash
redis-cli keys *
1) "test2"
2) "test1"
3) "test3"
```

### 📝 SET

```bash
redis-cli set api_key test_api_key
OK
```

### 📖 GET

```bash
redis-cli get api_key
"test_api_key"
```

```bash
redis-cli get test1
"test"
```

```bash
redis-cli get test
(nil)
```

### ⚙️ CONFIG

```bash
redis-cli config get dir
1) "dir"
2) "data"
```

---

## 📄 License

This project is licensed under the [MIT License](license.md). See the LICENSE file for details.
