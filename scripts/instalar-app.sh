#!/bin/sh
# Compila en release, (re)instala "Rompecabezas Montessori.app" en /Applications
# y deja el comando `montessori-puzzles` en ~/.cargo/bin apuntando a la app.
# Al final borra target/ para no dejar artefactos; con --conservar-target
# lo mantiene y la próxima compilación es incremental.
set -eu

RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
APP="/Applications/Rompecabezas Montessori.app"
BINARIO="montessori-puzzles"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$RAIZ/Cargo.toml" | head -1)"

conservar_target=0
for arg in "$@"; do
	case "$arg" in
	--conservar-target) conservar_target=1 ;;
	*) echo "Uso: $0 [--conservar-target]" >&2; exit 2 ;;
	esac
done

echo "==> Compilando $BINARIO $VERSION (release)"
cargo build --release --manifest-path "$RAIZ/Cargo.toml"

if pgrep -f "$APP/Contents/MacOS/$BINARIO" >/dev/null; then
	echo "==> Cerrando la app abierta"
	pkill -f "$APP/Contents/MacOS/$BINARIO" || true
	sleep 1
fi

echo "==> Instalando en $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp "$RAIZ/target/release/$BINARIO" "$APP/Contents/MacOS/"
cat >"$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>Rompecabezas Montessori</string>
	<key>CFBundleDisplayName</key><string>Rompecabezas Montessori</string>
	<key>CFBundleIdentifier</key><string>com.aleprieto.montessori-puzzles</string>
	<key>CFBundleVersion</key><string>$VERSION</string>
	<key>CFBundleShortVersionString</key><string>$VERSION</string>
	<key>CFBundleExecutable</key><string>$BINARIO</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>LSMinimumSystemVersion</key><string>11.0</string>
	<key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
codesign --force --sign - "$APP"
codesign --verify "$APP"

mkdir -p "$HOME/.cargo/bin"
ln -sf "$APP/Contents/MacOS/$BINARIO" "$HOME/.cargo/bin/$BINARIO"

if [ "$conservar_target" -eq 0 ]; then
	echo "==> Borrando target/"
	cargo clean --manifest-path "$RAIZ/Cargo.toml"
fi

echo "==> Listo: $("$APP/Contents/MacOS/$BINARIO" --version)"
