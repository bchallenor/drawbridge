with import <nixpkgs> {};

let
  # Assumes rust-overlay from nixpkgs-mozilla is installed
  rust = latest.rustChannels.stable.rust.override {
    extensions = ["rust-src"];
  };
  rustPlatform = recurseIntoAttrs (makeRustPlatform {
    rustc = rust;
    cargo = rust;
  });

in
callPackage ./package.nix {
  inherit rustPlatform;
}
