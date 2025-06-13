# Nix flake for reproducible Rust development and builds.
# - Uses `crane` for Rust builds and incremental caching.
# - Uses `rust-overlay` for toolchain from `rust-toolchain` file.
# - Provides a `devShell` with all dev dependencies and `pre-commit`.
# - Exposes the built crate as the default package.

{
  # Flake inputs: build helpers, overlays, and package set
  inputs = {
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  # Flake outputs: build and dev environments for each system
  outputs = { self, nixpkgs, crane, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # Import nixpkgs with Rust overlay
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        inherit (pkgs) lib;

        # Rust toolchain and build dependencies
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        nativeBuildInputs = with pkgs; [ rustToolchain pkg-config ];
        buildInputs = with pkgs; [ openssl ];

        # Prepare source and build args using crane
        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;
        commonArgs = {
          inherit src nativeBuildInputs buildInputs;
          strictDeps = true;
          doCheck = false;
        };

        # Build Rust dependencies and crate
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        my-crate = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });

      in {
        # Expose built crate and devShell
        packages = { default = my-crate; };

        devShells.default = with pkgs;
          mkShell {
            inherit nativeBuildInputs;
            buildInputs = buildInputs ++ [ pre-commit ];
            shellHook = ''
              export PATH=${rustToolchain}/bin:$PATH
            '';
          };
      });
}
