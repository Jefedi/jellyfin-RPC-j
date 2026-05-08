#!/bin/sh
set -e

# Le volume nomme /run/discord-ipc est cree par docker en root:root.
# On le rend ecrivable par node (uid/gid 1000), qui doit aussi correspondre
# a l'uid de jellyfin-rpc pour qu'il puisse se connecter au socket.
mkdir -p "$XDG_RUNTIME_DIR"
chown -R node:node "$XDG_RUNTIME_DIR"
chmod 755 "$XDG_RUNTIME_DIR"

# Si un socket stale traine d'un crash precedent, arrpc echouerait avec EADDRINUSE.
rm -f "$XDG_RUNTIME_DIR"/discord-ipc-*

exec su-exec node:node "$@"
