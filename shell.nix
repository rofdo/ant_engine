{pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
      pkg-config
    ];
    buildInputs = with pkgs; [
      wayland
      libxkbcommon
      xorg.libX11
      xorg.libXcursor
      xorg.libXrandr
      xorg.libXi
      libglvnd
      cmake
      vulkan-tools
      vulkan-headers
      vulkan-loader
      vulkan-validation-layers
      pkg-config 
      libGL
    ];

    LD_LIBRARY_PATH="${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib:${pkgs.xorg.libX11}/lib:${pkgs.xorg.libXcursor}/lib:${pkgs.xorg.libXrandr}/lib:${pkgs.xorg.libXi}/lib:${pkgs.libglvnd}/lib:${pkgs.libglvnd}/lib:${pkgs.libglvnd}/lib:${pkgs.cmake}/lib:${pkgs.vulkan-tools}/lib:${pkgs.vulkan-headers}/lib:${pkgs.vulkan-loader}/lib:${pkgs.vulkan-validation-layers}/lib:${pkgs.pkg-config}/lib:${pkgs.libGL}/lib";
}

