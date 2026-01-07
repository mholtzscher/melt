{
  description = "Minimal flake with just nixpkgs";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    {
      packages.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.hello;
    };
}
