{ pkgs ? import <nixpkgs> {} }:
let manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in pkgs.rustPlatform.buildRustPackage rec {
  pname = "geni";
  version = manifest.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = pkgs.lib.sources.cleanSource ./.;
  # The test is running before we build nix 
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
}
