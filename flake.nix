{
  inputs = {
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, crane, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        inherit (pkgs) lib;
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        nativeBuildInputs = with pkgs; [ rustToolchain pkg-config ];
        buildInputs = with pkgs; [ openssl ];
        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;
        commonArgs = {
          inherit src nativeBuildInputs buildInputs;
          strictDeps = true;
          doCheck = false;
        };
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs);
        my-crate =
          craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
      in {
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
