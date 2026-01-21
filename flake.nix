{
  description = "Minimalistic web UI for Taskwarrior";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = { # Rust toolchain
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ fenix.overlays.default ];
      };
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          lib = pkgs.lib;

          frontend = pkgs.buildNpmPackage {
            pname = "taskwarrior-web-frontend";
            version = "0.0.0";

            src = ./frontend;

            npmDepsHash = "sha256-Ul/gE/XEAehckV1Qj6Qwyy7QTx4sju6/0Omb3srQSgQ=";

            # The build commands from build.rs
            buildPhase = ''
              runHook preBuild # Install dependencies

              mkdir -p dist

              node_modules/.bin/tailwindcss -i css/style.css -o dist/style.css

              # The original config references 'frontend/src/main.ts' but we're already in frontend/
              substituteInPlace rollup.config.js \
                --replace-fail 'frontend/src/main.ts' 'src/main.ts'

              node_modules/.bin/rollup rollup.config.js

              cp -r templates dist/

              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              mkdir -p $out
              cp -r dist $out/
              runHook postInstall
            '';

            dontNpmBuild = true;
          };

          rustToolchain = pkgs.fenix.complete.withComponents [
            "cargo"
            "clippy"
            "rust-src"
            "rustc"
            "rustfmt"
          ];

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          manifest = (pkgs.lib.importTOML ./Cargo.toml).package;

          taskwarrior-web = rustPlatform.buildRustPackage {
            pname = manifest.name;
            version = manifest.version;

            src = ./.;

            cargoHash = "sha256-8eudwEVFDCmFbereV3f8ABOaXjjg4XxuAheO6dNPLqA=";

            nativeBuildInputs = [ pkgs.makeWrapper ];

            buildInputs = with pkgs; [
              sqlite
            ];

            # Some tests require a writable HOME directory which isn't available in the sandbox
            doCheck = false;
            
            preBuild = ''
              rm build.rs # Skip the build.rs entirely since we built frontend separately
              mkdir -p $out/dist
              cp -r ${frontend}/* $out
            '';

            postFixup = ''
              wrapProgram $out/bin/taskwarrior-web \
                --set TWK_STATICS_DIR "$out/dist"
            '';

            meta = with lib; {
              description = "Minimalistic web UI for Taskwarrior";
              homepage = "https://github.com/tmahmood/taskwarrior-web.git";
              license = licenses.mit;
              mainProgram = "taskwarrior-web";
            };
          };
        in
        {
          inherit frontend taskwarrior-web;
          default = taskwarrior-web;
        }
      );
    };
}
