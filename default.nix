{ pkgs ? import <nixpkgs> {} }:
let manifest = (pkgs.lib.importTOML ./Cargo.toml).package;

let
  gitignoreSource =
    if pkgs.gitignoreSrc != null
    then pkgs.gitignoreSrc.gitignoreSource
    else (import (fetchFromGitHub {
      owner = "hercules-ci";
      repo = "gitignore";
      rev = "c4662e662462e7bf3c2a968483478a665d00e717";
      sha256 = "0jx2x49p438ap6psy8513mc1nnpinmhm8ps0a4ngfms9jmvwrlbi";
    }) { inherit lib; }).gitignoreSource;
in

in pkgs.rustPlatform.buildRustPackage rec {
  pname = "geni";
  version = manifest.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = gitignoreSource ./.;
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
