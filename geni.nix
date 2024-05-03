{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation rec {
  pname = "geni";
  version = "1.0.5";

  src = pkgs.fetchFromGitHub {
    owner = "emilpriver";
    repo = pname;
    rev = "v${version}";
    sha256 = "sha256-pjAP4AOR2sUF+PIrZhvUSZVM+DuKLU56ikfHCxrLun8="; 
  };

  environment.variables = rec {
    CARGO_NET_GIT_FETCH_WITH_CLI = true;
  };

  nativeBuildInputs = with pkgs; [
    rustc
    cargo
    pkg-config
  ];

  buildPhase = ''
    cargo build --release
  '';

  installPhase = ''
    mkdir -p $out/bin
    cp target/release/geni $out/bin/
  '';


  meta = with pkgs.lib; {
    description = "Standalone database migration tool which works for Postgres, MariaDB, MySQL, Sqlite and LibSQL(Turso).";
    longDescription = ''
      Standalone database migration tool which works for Postgres, MariaDB, MySQL, Sqlite and LibSQL(Turso).
    '';
    homepage = "https://github.com/emilpriver/geni";
    changelog = "https://github.com/emilpriver/geni/releases/tag/v${version}";
    license = licenses.mit;
    maintainers = [ maintainers.emilpriver ];
    platforms = platforms.all;
  };
}
