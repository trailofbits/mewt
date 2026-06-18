{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
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

      mkToolchain = p: let
        fenixPkgs = fenix.packages.${p.stdenv.buildPlatform.system};
      in with fenixPkgs; combine [
        stable.completeToolchain
        fenixPkgs.targets.${targets.x86_64-linux}.stable.rust-std
        fenixPkgs.targets.${targets.aarch64-linux}.stable.rust-std
      ];
      toolchain = mkToolchain pkgs;

      craneBuild = targetSystem: let
        pkgsCross = mkPkgs targetSystem;
        isNativeBuild = targetSystem == null || targetSystem == buildSystem;
        craneLib = (crane.mkLib pkgsCross).overrideToolchain mkToolchain;
        src = ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
          doCheck = false;

          buildInputs = [
            toolchain
          ] ++ (if isNativeBuild then [
            pkgsCross.sqlite.dev
          ] else [
            pkgsCross.sqlite-static
          ]);

          nativeBuildInputs = with pkgs; [
            pkgs.stdenv.cc # rust dependency build scripts must run on the build system
            pkgsCross.pkg-config
          ];

          # Native/Cross settings
        } // (if isNativeBuild then {
          PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" [
            pkgsCross.sqlite.dev
          ];
        } else rec {
          TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";
          PKG_CONFIG_ALL_STATIC = "1";
          PKG_CONFIG_ALLOW_CROSS = "1";
          PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" [
            pkgsCross.sqlite-static.dev
            pkgsCross.zlib.static
          ];
          CARGO_BUILD_TARGET = targets.${targetSystem};
          CARGO_BUILD_RUSTFLAGS = pkgs.lib.concatStringsSep " " [
            # Tells Cargo to enable static compilation
            "-C" "target-feature=+crt-static"
            # https://github.com/rust-lang/cargo/issues/4133
            "-C" "linker=${TARGET_CC}"
            "-L" "${pkgsCross.sqlite-static.dev}/lib"
            "-L" "${pkgsCross.zlib.static}/lib"
          ];
        });

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in
        craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
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
        });

    in rec {

      packages = {
        default = packages.mewt;
        mewt = craneBuild null;
        mewt-x86_64-linux = craneBuild "x86_64-linux";
        mewt-aarch64-linux = craneBuild "aarch64-linux";
        mewt-aarch64-darwin = craneBuild "aarch64-darwin";
      };

      devInputs = with pkgs; [
        actionlint
        cargo-watch
        just
        libiconv
        openssl
        pkg-config
        rlwrap
        rustup
        sqlite
        sqlx-cli
        toolchain
        typos
      ];

      devShells = {
        default = pkgs.mkShell {
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath devInputs;
          buildInputs = devInputs;
          shellHook = ''
            export CARGO_HOME=$(pwd)/.cargo
            export PATH="$PATH:$CARGO_HOME/bin"
          '';
        };
      };

    }
  );

}
