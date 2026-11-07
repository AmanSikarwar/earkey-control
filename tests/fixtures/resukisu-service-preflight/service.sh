#!/system/bin/sh
set -eu

if [ "${KSU:-}" != true ] ||
   [ "${BAKASU:-}" != true ] ||
   [ "${KSU_MODULE:-}" != earkey_control_preflight ] ||
   [ "${KSU_VER_CODE:-}" != 35257 ] ||
   [ "${KSU_KERNEL_VER_CODE:-}" != 35240 ] ||
   [ "${KSU_UAPI_VER:-}" != 6 ] ||
   [ "$(id -u)" != 0 ]; then
    exit 1
fi

case "$0" in
    */service.sh) cd "${0%/*}" ;;
    *) exit 1 ;;
esac

for flag in disable remove; do
    if [ -e "$flag" ] || [ -L "$flag" ]; then
        exit 0
    fi
done

umask 077
# Reserve the only attempt before checking or starting the payload.
mkdir -m 0700 .preflight 2>/dev/null || exit 0
(
    status=1
    if [ -f ./probe.sha256 ] && [ ! -L ./probe.sha256 ] &&
       [ "$(stat -c '%s' ./probe.sha256)" = 65 ] &&
       [ -f ./earkey-probe ] && [ ! -L ./earkey-probe ] && [ -x ./earkey-probe ] &&
       IFS= read -r expected < ./probe.sha256 &&
       actual=$(timeout -s KILL 5 /system/bin/sha256sum ./earkey-probe) &&
       [ "${actual%% *}" = "$expected" ]; then
        printf 'launcher=resukisu_service\n'
        printf 'artifact_sha256=%s\n' "$expected"
        status=0
        timeout -s KILL 5 ./earkey-probe self-check || status=$?
    else
        printf 'preflight=digest_mismatch\n'
    fi
    printf 'service_probe_exit=%s\n' "$status"
) > .preflight/result.tmp 2>&1
mv .preflight/result.tmp .preflight/result
