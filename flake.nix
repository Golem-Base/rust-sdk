{
  description = "A Rust SDK for interacting with GolemBase.";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self
    , nixpkgs
    , crane
    , fenix
    , flake-utils
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      inherit (pkgs) lib;
      pkgs = nixpkgs.legacyPackages.${system};

      rustToolchain = fenix.packages.${system}.fromToolchainFile {
        file = ./.rust-toolchain.toml;
        sha256 = "sha256-Qxt8XAuaUR2OMdKbN4u8dBJOhSHxS+uS06Wl9+flVEk=";
      };

      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
      src =
        let
          markdownFilter = path: _type: builtins.match ".*md$" path != null;
          markdownOrCargo = path: type:
            (markdownFilter path type) || (craneLib.filterCargoSources path type);
        in
        lib.cleanSourceWith {
          src = ./.;
          filter = markdownOrCargo;
          name = "source";
        };

      commonArgs = {
        inherit src;
        strictDeps = true;

        # TODO: Check dependencies for rustls, we can potentially
        # remove the dependency on pkg-config and openssl
        nativeBuildInputs = with pkgs; [ pkg-config ];

        buildInputs = with pkgs; [ openssl ];
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      golem-base-sdk = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
        doCheck = false; # The tests don't work in the nix sandbox
      });
    in
    {
      # Additional cargo checks can be found at https://github.com/ipetkov/crane.
      #
      # These derivations are inherited by the `devShell`.
      checks = {
        inherit golem-base-sdk;

        cargo-clippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

        cargo-fmt = craneLib.cargoFmt {
          inherit src;
        };

        taplo-fmt = craneLib.taploFmt {
          src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
        };

        cargo-doc = craneLib.cargoDoc (commonArgs // {
          inherit cargoArtifacts;
          cargoDocExtraArgs = "--no-deps --workspace";
          # This can be commented out or tweaked as necessary, e.g. set to
          # `--deny rustdoc::broken-intra-doc-links` to only enforce that lint
          env.RUSTDOCFLAGS = "--deny warnings";
        });
      };

      packages = {
        inherit golem-base-sdk;
        default = golem-base-sdk;
      };

      devShells.default = craneLib.devShell {
        checks = self.checks.${system};
        packages = with pkgs; [
          pre-commit
          nil
          nixpkgs-fmt
        ];
      };

      formatter = pkgs.nixpkgs-fmt;
    });
}
