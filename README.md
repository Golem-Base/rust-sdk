# GolemBase SDK

This is part of the [Golem Base](https://github.com/Golem-Base) project, which is designed as a Layer2 Network deployed on Ethereum, acting as a gateway to various Layer 3 Database Chains (DB-Chains).
For an overview of Golem Base, **check out our [Litepaper](https://golem-base.io/wp-content/uploads/2025/03/GolemBase-Litepaper.pdf)**.

This SDK allows you to use [GolemBase](https://github.com/Golem-Base) from Rust, it is available on [crates.io](https://crates.io/crates/golem-base-sdk), alng with its [generated documentation](https://docs.rs/golem-base-sdk). We provide an [example application](https://github.com/Golem-Base/rust-sdk/tree/main/demo) to showcase how you can use this SDK.

For **getting up and running quickly**, we recommend the following two steps:
1. Start golembase-op-geth through its [`docker-compose`](https://github.com/Golem-Base/golembase-op-geth/blob/main/RUN_LOCALLY.md) ;
2. [Install the demo CLI](https://github.com/Golem-Base/golembase-demo-cli?tab=readme-ov-file#installation) and [create a user](https://github.com/Golem-Base/golembase-demo-cli?tab=readme-ov-file#quickstart), or build the [actual CLI](https://github.com/Golem-Base/golembase-op-geth/blob/main/cmd/golembase/README.md) as it's included in the `golembase-op-geth` repository.

When you create a user, it will generate a private key file called `private.key` and store it in the standard folder as per the [XDG specification](https://specifications.freedesktop.org/basedir-spec/latest/):
- `~/.config/golembase/` on **Linux**  
- `~/Library/Application Support/golembase/` on **macOS**  
- `%LOCALAPPDATA%\golembase\` on **Windows**  

You will also need to fund the account, you can do it with: `golembase-demo-cli account fund 10`
