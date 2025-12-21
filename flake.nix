{
  description = "`gitoxide` compiled in a nix shell, using `crane` and `flakebox`.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flakebox = {
      url = "github:rustshop/flakebox";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
    };
  };

  outputs = { self, nixpkgs, flakebox, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (arch: let
      pkgs = import nixpkgs { system = arch; };
      flakeboxLib = flakebox.lib.mkLib pkgs { };

      rustSrc = flakeboxLib.filterSubPaths {
        root = builtins.path { name = "gitoxide"; path = ./.; };
        paths = [ "Cargo.toml" "Cargo.lock" ".cargo"

          "etc"
          "examples"
          "gitoxide-core"
          "gix"
          "gix-actor"
          "gix-archive"
          "gix-attributes"
          "gix-bitmap"
          "gix-blame"
          "gix-chunk"
          "gix-command"
          "gix-commitgraph"
          "gix-config"
          "gix-config-value"
          "gix-credentials"
          "gix-date"
          "gix-diff"
          "gix-dir"
          "gix-discover"
          "gix-features"
          "gix-fetchhead"
          "gix-filter"
          "gix-fs"
          "gix-fsck"
          "gix-glob"
          "gix-hash"
          "gix-hashtable"
          "gix-ignore"
          "gix-index"
          "gix-lfs"
          "gix-lock"
          "gix-macros"
          "gix-mailmap"
          "gix-merge"
          "gix-negotiate"
          "gix-note"
          "gix-object"
          "gix-odb"
          "gix-pack"
          "gix-packetline"
          "gix-packetline-blocking"
          "gix-path"
          "gix-pathspec"
          "gix-prompt"
          "gix-protocol"
          "gix-quote"
          "gix-rebase"
          "gix-ref"
          "gix-refspec"
          "gix-revision"
          "gix-revwalk"
          "gix-sec"
          "gix-sequencer"
          "gix-shallow"
          "gix-status"
          "gix-submodule"
          "gix-tempfile"
          "gix-tix"
          "gix-trace"
          "gix-transport"
          "gix-traverse"
          "gix-tui"
          "gix-url"
          "gix-utils"
          "gix-validate"
          "gix-worktree"
          "gix-worktree-state"
          "gix-worktree-stream"
          "src"
          "tests"

        ];
      };

      legacyPackages = (flakeboxLib.craneMultiBuild { }) (craneLib': let
          craneLib = with pkgs; (craneLib'.overrideArgs {
            pname = "gitoxide";
            src = rustSrc;
            buildInputs = [ openssl.dev ];
            nativeBuildInputs = [ pkg-config ];
          });
        in rec {
          workspaceDeps = craneLib.buildWorkspaceDepsOnly { };
          workspaceBuild = craneLib.buildWorkspace { cargoArtifacts = workspaceDeps; };
          gitoxide = craneLib.buildPackage { };
        });
    in {
      inherit legacyPackages;
      packages.default = legacyPackages.gitoxide;
      devShells = flakeboxLib.mkShells {
        packages = [ ];
      };
  });
}
