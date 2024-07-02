{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource (craneLib.path ./.);
        cargoTomlInfo = craneLib.crateNameFromCargoToml { cargoToml = ./generator/Cargo.toml; };

        commonArgs = {
          inherit src;
          inherit (cargoTomlInfo) pname version;
          strictDeps = true;
          nativeBuildInputs = [ pkgs.rust-bin.nightly.latest.default ];
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        generator = (craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        })).overrideAttrs {
          meta.mainProgram = "generator";
        };

        clippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

        fmt = craneLib.cargoFmt commonArgs;

        test = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
        });

        site = pkgs.stdenv.mkDerivation rec {
          name = "site";
          version = cargoTomlInfo.version;
          src = ./.;
          nativeBuildInputs = [ generator ];
          buildPhase = ''
            mkdir -p $out/dist
            ${pkgs.lib.getExe generator} -b $out/dist -s ${./static}
          '';
        };
      in {
        checks = {
          inherit generator site clippy fmt test;
        };

        packages = {
          inherit generator site;
          default = site;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ generator ];
          nativeBuildInputs = [ pkgs.caddy pkgs.cargo-watch generator ];
        };
      }
    );
}
