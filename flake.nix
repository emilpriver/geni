{
  description = "A very basic flake with integrated Rust project geni";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # Using unstable channel for packages
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      # Importing the package configuration from previously separate default.nix
      geni = let
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
      in pkgs.rustPlatform.buildRustPackage rec {
        pname = "geni";
        version = manifest.version;
        cargoLock.lockFile = ./Cargo.lock;
        src = pkgs.lib.sources.cleanSource ./.;
        doCheck = false;

        buildPhase = ''
          cargo build --release --locked
        '';

        installPhase = ''
          mkdir -p $out/bin
          cp target/release/geni $out/bin/
        '';

        meta = with pkgs.lib; {
          description = manifest.description;
          longDescription = ''
            Standalone database migration tool which works for Postgres, MariaDB, MySQL, Sqlite and LibSQL(Turso).
          '';
          homepage = manifest.repository;
          changelog = "${manifest.repository}/releases/tag/v${version}";
          license = licenses.mit;
          maintainers = [ maintainers.emilpriver ];
          platforms = platforms.all;
        };
      };
    in rec {
      packages.geni = geni;

      legacyPackages = packages;

      defaultPackage = packages.geni;

      devShell = pkgs.mkShell {
        CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";

        buildInputs = with pkgs; [ cargo rustc git ];
      };
    });
}

