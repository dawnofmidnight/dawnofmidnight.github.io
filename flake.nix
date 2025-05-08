{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { flake-utils, nixpkgs, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        site = pkgs.callPackage ./default.nix { };
      in
      {
        checks = { inherit site; };

        packages = {
          inherit site;
          default = site;
        };

        devShells.default = pkgs.mkShellNoCC {
          packages = [
            pkgs.tinymist
            pkgs.typst
            pkgs.typstyle
          ];
          TYPST_FEATURES = "html";
        };
      }
    );
}
