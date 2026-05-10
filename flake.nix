{
  description = "Simple flake with a devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self, nixpkgs, flake-utils }:
    {
      templates.default = {
        path = ./.;
        description = "Simple flake with a devshell";
      };
    } // flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      devShells.default = import ./nix/devshell.nix { inherit pkgs; };
      formatter = pkgs.nixpkgs-fmt;
    });
}
