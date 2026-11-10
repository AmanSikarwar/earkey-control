#!/system/bin/sh
# Stops only this module's verified daemon and workers, then removes the private store.
case "$0" in
    */uninstall.sh) cd "${0%/*}" || exit 0 ;;
    *) exit 0 ;;
esac
binary=$(readlink -f ./bin/earkey-worker 2>/dev/null)

ours() {
    for dir in /proc/[0-9]*; do
        [ "$(readlink "$dir/exe" 2>/dev/null)" = "$binary" ] || continue
        command=$(tr '\0' ' ' < "$dir/cmdline" 2>/dev/null)
        case "$command" in
            *"earkey-worker $1 "*|*"earkey-worker $1") echo "${dir#/proc/}" ;;
        esac
    done
}

if [ -n "$binary" ]; then
    for pid in $(ours daemon); do kill -TERM "$pid" 2>/dev/null; done
    i=0
    while [ -n "$(ours daemon)" ] && [ "$i" -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
    for kind in daemon run; do
        for pid in $(ours "$kind"); do kill -KILL "$pid" 2>/dev/null; done
    done
fi

storage=/data/adb/earkey_control
if [ -d "$storage" ] && [ ! -L "$storage" ]; then
    rm -rf "$storage"
fi
