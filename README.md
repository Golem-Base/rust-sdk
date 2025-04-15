# GolemBase SDK

This is part of the [Golem Base](https://github.com/Golem-Base) project, which is designed as a Layer2 Network deployed on Ethereum, acting as a gateway to various Layer 3 Database Chains (DB-Chains).

> **For an overview of Golem Base, check out our [Litepaper](https://golem-base.io/wp-content/uploads/2025/03/GolemBase-Litepaper.pdf).**

This SDK allows you to use [GolemBase](https://github.com/Golem-Base) from Rust. It is available on [crates.io](https://crates.io/crates/golem-base-sdk).

We also publish [generated documentation](https://docs.rs/golem-base-sdk).

The repository contains an [example application](https://github.com/Golem-Base/rust-sdk/tree/main/example) to showcase how you can use this SDK.

**Tip:** For getting up and running quickly, we recommend the following two steps:

1. Start golembase-op-geth through its [docker-compose](https://github.com/Golem-Base/golembase-op-geth/blob/main/RUN_LOCALLY.md).

2. [Install the demo CLI](https://github.com/Golem-Base/golembase-demo-cli?tab=readme-ov-file#installation) and [create a user](https://github.com/Golem-Base/golembase-demo-cli?tab=readme-ov-file#quickstart).

(Note: As an alternative to installing the demo CLI, you can build the [actual CLI](https://github.com/Golem-Base/golembase-op-geth/blob/main/cmd/golembase/README.md) as it's included in the golembase-op-geth repo.)

When you create a user, it will generate a private key file called `private.key` and store it in:

- `~/.config/golembase/` on **Linux**  
- `~/Library/Application Support/golembase/` on **macOS**  
- `%LOCALAPPDATA%\golembase\` on **Windows**  

(This is a standard folder as per the [XDG specification](https://specifications.freedesktop.org/basedir-spec/latest/).)

You will also need to fund the account. You can do so by typing:

```
golembase-demo-cli account fund 10
```