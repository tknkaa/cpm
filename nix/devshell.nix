{ pkgs }:
pkgs.mkShell {
  packages = with pkgs; [
    go
  ];
}
