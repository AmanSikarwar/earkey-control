#!/system/bin/sh
# Starts the supervisor daemon once per boot, only for the reviewed binary.
case "$0" in
    */service.sh) cd "${0%/*}" || exit 0 ;;
    *) exit 0 ;;
esac
storage=/data/adb/earkey_control
[ -d "$storage" ] && [ ! -L "$storage" ] || exit 0
[ -f ./bin/earkey-worker ] && [ ! -L ./bin/earkey-worker ] || exit 0
[ -f ./bin/earkey-worker.sha256 ] && [ ! -L ./bin/earkey-worker.sha256 ] || exit 0
IFS= read -r expected < ./bin/earkey-worker.sha256 || exit 0
actual=$(timeout -s KILL 5 /system/bin/sha256sum ./bin/earkey-worker) || exit 0
[ "${actual%% *}" = "$expected" ] || exit 0
./bin/earkey-worker daemon </dev/null >/dev/null 2>&1 &
