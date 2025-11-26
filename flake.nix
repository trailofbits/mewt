{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    naersk.inputs.fenix.follows = "fenix";
  };

  outputs = inputs: with inputs;
    utils.lib.eachDefaultSystem (buildSystem: let
      libc = "musl"; # "gnu" or "musl"
      targets = {
        "x86_64-linux" = "x86_64-unknown-linux-${libc}";
        "aarch64-linux" = "aarch64-unknown-linux-${libc}";
        "aarch64-darwin" = "aarch64-apple-darwin";
      };

      # The nixpkgs cache doesn't have any packages where cross-compiling has
      # been enabled, even if the target platform is actually the same as the
      # build platform (and therefore it's not really cross-compiling). So we
      # only set up the cross-compiling config if the target platform is
      # different.
      mkPkgs = targetSystem: import nixpkgs ({
        system = buildSystem;
        # Only use static stdenv for cross compilation
        stdenv = if targetSystem != null && targetSystem != buildSystem 
                 then nixpkgs.pkgsStatic.stdenv 
                 else nixpkgs.stdenv;
        config.allowUnfree = true;
        overlays = [
          (self: super: {
            sqlite-static = super.sqlite.overrideAttrs (oldAttrs: {
              configureFlags = oldAttrs.configureFlags or [] ++ [
                "--enable-static"
                "--disable-shared"
              ];
            });
            zlib = super.zlib.override {
              static = true;
            };
          })
        ];
      } // (if targetSystem == null || targetSystem == buildSystem then {} else {
        inherit libc;
        crossSystem.config = targets.${targetSystem};
      }));

      pkgs = mkPkgs null;

      fenixPkgs = fenix.packages.${buildSystem};
      toolchain = with fenixPkgs; combine [
        stable.completeToolchain
        fenixPkgs.targets.${targets.x86_64-linux}.stable.rust-std
        fenixPkgs.targets.${targets.aarch64-linux}.stable.rust-std
      ];

      naerskBuild = { targetSystem, crate }: let
        pkgsCross = mkPkgs targetSystem;
        isNativeBuild = targetSystem == null || targetSystem == buildSystem;
      in (naersk.lib.${buildSystem}.override {
        cargo = toolchain;
        rustc = toolchain;
      }).buildPackage(rec {
        src = ./.;
        strictDeps = true;
        doCheck = false;
        release = true;
        cargoBuildOptions = x: x ++ [ "-p" crate ];
        postInstall = if targetSystem == null then "" else ''
          cd "$out"/bin
          for f in $(ls); do
            if ext="$(echo "$f" | grep -oP '\\.[a-z]+$')"; then
              base="$(echo "$f" | cut -d. -f1)"
              mv "$f" "$base-${targetSystem}$ext"
            else
              mv "$f" "$f-${targetSystem}"
            fi
          done
        '';

        buildInputs = [
          toolchain
        ] ++ (if isNativeBuild then [
          pkgsCross.sqlite.dev
        ] else [
          pkgsCross.sqlite-static
        ]);

        nativeBuildInputs = with pkgs; [
          pkgs.stdenv.cc # rust dependency build scripts must run on the build system
        ] ++ (if isNativeBuild then [
          pkgsCross.pkg-config
        ] else [
          pkgsCross.pkg-config
        ]);

        # Cross-compilation specific settings
      } // (if isNativeBuild then {
        # Native build settings
        PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" [
          pkgsCross.sqlite.dev
        ];
      } else rec {
        # Cross-compilation settings
        TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";
        PKG_CONFIG_ALL_STATIC = "1";
        PKG_CONFIG_ALLOW_CROSS = "1";
        PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" [
          pkgsCross.sqlite-static.dev
          pkgsCross.zlib.static
        ];
        CARGO_BUILD_TARGET = targets.${targetSystem};
        CARGO_BUILD_RUSTFLAGS = [
          # Tells Cargo to enable static compilation
          "-C" "target-feature=+crt-static"
          # https://github.com/rust-lang/cargo/issues/4133
          "-C" "linker=${TARGET_CC}"
          "-L" "${pkgsCross.sqlite-static.dev}/lib"
          "-L" "${pkgsCross.zlib.static}/lib"
        ];
      }));

    in rec {

      packages = {
        default = packages.mewt;
        mewt = naerskBuild { targetSystem = null; crate = "mewt"; };
        mewt-x86_64-linux = naerskBuild { targetSystem = "x86_64-linux"; crate = "mewt"; };
        mewt-aarch64-linux = naerskBuild { targetSystem = "aarch64-linux"; crate = "mewt"; };
        mewt-aarch64-darwin = naerskBuild { targetSystem = "aarch64-darwin"; crate = "mewt"; };
      };

      devInputs = with pkgs; [
        cargo-watch
        libiconv
        openssl
        pkg-config
        rlwrap
        rustup
        sqlite
        sqlx-cli
        toolchain
      ];

      devShells = {
        default = pkgs.mkShell {
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath devInputs;
          buildInputs = devInputs;
          shellHook = ''
            export CARGO_HOME=$(pwd)/.cargo
            export PATH="$CARGO_HOME/bin:$PATH"
          '';
        };
      };

    }
  );

}
