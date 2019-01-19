{ rustPlatform, pkgconfig, openssl }:

rustPlatform.buildRustPackage {
  name = "drawbridge";

  src = fetchGit {
    url = ./.;
  };

  cargoSha256 = "137k6r4bzifkpmad7p66fkqdwnfaiysn0qaldw030cxh48nggjkx";

  nativeBuildInputs = [ pkgconfig ];
  buildInputs = [ openssl ];
}
