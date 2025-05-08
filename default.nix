{ lib, stdenv, typst, typstyle }:
stdenv.mkDerivation {
  name = "site";
  src = ./.;
  nativeBuildInputs = [
    typst
  ];
  buildPhase = ''
    rm -rf build
    mkdir -p build
    for file in $(find src -type f -name "*.typ");
    do
      output="''${file#"src/"}"
      output="build/''${output%.typ}.html"
      typst compile --format html --features html "$file" "$output"
    done
  '';
  installPhase = ''
    mkdir -p $out/site
    cp -r build/. $out/site
    cp -r static/. $out/site
  '';
  checkPhase = ''
    ${lib.getExe typstyle} format-all --check
  '';
}
