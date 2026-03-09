{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

    nativeBuildInputs = with pkgs; [
      rustc
      clippy
      cargo
      meson
      ninja
      pkg-config
      blueprint-compiler
      appstream-glib
      desktop-file-utils
      wrapGAppsHook4
      rustPlatform.cargoSetupHook
      xdg-desktop-portal
    ];

    buildInputs = with pkgs; [
      libusb1
      gtk4
      libadwaita
      libgudev
    ];

    pname = "lift";
    version = "0.1.0";
    src = ./.;
  in {
    devShells.${system}.default = pkgs.mkShell {
      inherit nativeBuildInputs buildInputs;
      RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      
      packages = with pkgs; [
        flatpak-builder
        rust-analyzer
        rustfmt
        d-spy
        ghex
      ];
    };

    packages.${system}.default = pkgs.stdenv.mkDerivation {
      name = pname;
      inherit version src;

      cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
        inherit pname version src;
        hash = "sha256-zige76qMsVhogo9b3fQVxqONju3KXwiUAe9vScOes1w=";
      };

      inherit nativeBuildInputs buildInputs;
    };
  };
}
