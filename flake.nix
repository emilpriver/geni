{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:emilpriver/geni";
    gitignore = { url = "github:emilpriver/gitignore.nix"; flake = false; };
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, ... }:
  flake-utils.lib.eachSystem [ "x86_64-linux" ] (system: let
    pkgs = nixpkgs.legacyPackages.${system};
    gitignoreSrc = pkgs.callPackage inputs.gitignore { };
  in rec {
    packages.geni = pkgs.callPackage ./default.nix { inherit gitignoreSrc; };

    legacyPackages = packages;

    defaultPackage = packages.geni;

    devShell = pkgs.mkShell {
      CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";

      buildInputs = with pkgs; [ cargo rustc git ];
    };
  });
}
