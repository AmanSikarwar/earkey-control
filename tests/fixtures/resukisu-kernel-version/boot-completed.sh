#!/system/bin/sh
set -eu
case "$0" in
    */boot-completed.sh) cd "${0%/*}" ;;
    *) exit 1 ;;
esac
exec /data/adb/ksu/bin/busybox sh "$PWD/service.sh" --boot-completed
