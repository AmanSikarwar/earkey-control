#!/system/bin/sh
set -eu

case "$0" in
    */service.sh) cd "${0%/*}" ;;
    *) exit 1 ;;
esac

case "${1:-}" in
    '') stage=service; attempt=.preflight ;;
    --boot-completed) stage=boot_completed; attempt=.boot-preflight ;;
    *) exit 1 ;;
esac
[ "$#" -le 1 ] || exit 1

for flag in disable remove; do
    if [ -e "$flag" ] || [ -L "$flag" ]; then
        exit 0
    fi
done

umask 077
# Reserve the only attempt before checking or starting the payload.
mkdir -m 0700 "$attempt" 2>/dev/null || exit 0
(
    uptime=
    if IFS=' ' read -r uptime _ 2>/dev/null < /proc/uptime && [ "${#uptime}" -le 20 ]; then
        case "$uptime" in
            *.*)
                whole=${uptime%%.*}; fraction=${uptime#*.}
                case "$whole" in
                    ''|*[!0123456789]*) uptime=unavailable ;;
                esac
                case "$fraction" in
                    ''|*[!0123456789]*) uptime=unavailable ;;
                esac
                ;;
            *) uptime=unavailable ;;
        esac
    else
        uptime=unavailable
    fi
    printf 'hook_uptime_seconds=%s\n' "$uptime"
    context=
    if IFS= read -r context 2>/dev/null < "/proc/$$/attr/current" || [ -n "$context" ]; then
        case "$context" in
            ''|*[!a-zA-Z0-9_:.,-]*) printf 'selinux_context=invalid\n' ;;
            *)
                if [ "${#context}" -le 128 ]; then
                    printf 'selinux_context=%s\n' "$context"
                else
                    printf 'selinux_context=invalid\n'
                fi
                ;;
        esac
    else
        printf 'selinux_context=unavailable\n'
    fi
    for ns in mnt uts; do
        ns_target=$(timeout -s KILL 5 /system/bin/readlink "/proc/$$/ns/$ns" 2>/dev/null) || ns_target=
        case "$ns_target" in
            "$ns":\[*\])
                ns_id=${ns_target#*\[}; ns_id=${ns_id%\]}
                case "$ns_id" in
                    ''|*[!0123456789]*) ns_id=unavailable ;;
                    *) [ "${#ns_id}" -le 20 ] || ns_id=unavailable ;;
                esac
                ;;
            *) ns_id=unavailable ;;
        esac
        printf '%s_namespace=%s\n' "$ns" "$ns_id"
    done
    # Inspect only this hook shell; unrelated descriptor targets are never printed.
    scan=complete; scanned=0
    driver_yes=0; driver_no=0; driver_unknown=0
    su_yes=0; su_no=0; su_unknown=0
    for fd in "/proc/$$/fd/"*; do
        scanned=$((scanned + 1))
        if [ "$scanned" -gt 16 ]; then scan=incomplete; break; fi
        if [ ! -L "$fd" ]; then scan=incomplete; continue; fi
        target=$(timeout -s KILL 1 /system/bin/readlink "$fd" 2>/dev/null) || { scan=incomplete; continue; }
        case "$target" in
            'anon_inode:[ksu_driver]') kind=driver ;;
            'anon_inode:[ksu_driver_su]') kind=su ;;
            *) continue ;;
        esac
        number=${fd##*/}; flags=; flag_count=0
        if [ -r "/proc/$$/fdinfo/$number" ]; then
            while IFS= read -r line; do
                case "$line" in
                    flags:*)
                        set -f
                        # flags fields require word splitting; pathname expansion is disabled.
                        # shellcheck disable=SC2086
                        set -- $line
                        set +f
                        flag_count=$((flag_count + 1))
                        [ "$#" = 2 ] && flags=$2 || flags=
                        ;;
                esac
            done < "/proc/$$/fdinfo/$number"
        fi
        close=unknown
        case "$flags" in
            ''|*[!01234567]*) ;;
            *)
                if [ "$flag_count" = 1 ] && [ "${#flags}" -le 10 ]; then
                    if [ $((0$flags & 02000000)) -ne 0 ]; then close=yes; else close=no; fi
                fi
                ;;
        esac
        case "$kind:$close" in
            driver:yes) driver_yes=$((driver_yes + 1)) ;;
            driver:no) driver_no=$((driver_no + 1)) ;;
            driver:unknown) driver_unknown=$((driver_unknown + 1)) ;;
            su:yes) su_yes=$((su_yes + 1)) ;;
            su:no) su_no=$((su_no + 1)) ;;
            su:unknown) su_unknown=$((su_unknown + 1)) ;;
        esac
    done
    printf 'fd_scan=%s\ndriver_cloexec_yes=%s\ndriver_cloexec_no=%s\ndriver_cloexec_unknown=%s\n' "$scan" "$driver_yes" "$driver_no" "$driver_unknown"
    printf 'driver_su_cloexec_yes=%s\ndriver_su_cloexec_no=%s\ndriver_su_cloexec_unknown=%s\n' "$su_yes" "$su_no" "$su_unknown"
) > "$attempt/context.tmp" 2>&1
mv "$attempt/context.tmp" "$attempt/context"
(
    printf 'hook=entered\n'
    status=1

    manager_path=/data/adb/ksud
    manager_sha=fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50
    expected_release='4.14.357-reforge-g9b21e3df6c0a'
    expected_build='#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026'
    expected_flags='0x160'
    expected_features='0x7'

    guard() {
        if [ "$2" != "$3" ]; then
            printf 'guard_failure=%s\n' "$1"
            return 1
        fi
    }

    valid_field() {
        field_value=$1; field_kind=$2
        case "$field_kind" in
            number)
                case "$field_value" in
                    ''|*[!0123456789]*) return 1 ;;
                esac
                [ "${#field_value}" -le 10 ]
                ;;
            full)
                [ -n "$field_value" ] && [ "${#field_value}" -le 128 ] || return 1
                case "$field_value" in
                    *[!A-Za-z0-9_.+@~:/-]*) return 1 ;;
                esac
                ;;
            flags)
                case "$field_value" in
                    0x*|0X*) ;;
                    *) return 1 ;;
                esac
                hex=${field_value#0x}; hex=${hex#0X}
                [ -n "$hex" ] && [ "${#hex}" -le 8 ] || return 1
                case "$hex" in
                    *[!0123456789abcdefABCDEF]*) return 1 ;;
                esac
                ;;
            bool)
                case "$field_value" in
                    true|false) ;;
                    *) return 1 ;;
                esac
                ;;
            token)
                [ -n "$field_value" ] && [ "${#field_value}" -le 32 ] || return 1
                case "$field_value" in
                    *[!A-Za-z0-9_.-]*) return 1 ;;
                esac
                ;;
            *) return 1 ;;
        esac
    }

    emit_info_field() {
        info_label=$1; info_value=$2; info_count=$3; info_kind=$4
        if [ "$info_count" -eq 0 ]; then
            printf '%s=missing\n' "$info_label"; info_status=invalid; return
        fi
        if [ "$info_count" -ne 1 ]; then
            printf '%s=duplicate\n' "$info_label"; info_status=invalid; return
        fi
        if [ "$info_kind" = full ] && [ -z "$info_value" ]; then
            printf '%s=unknown\n' "$info_label"; return
        fi
        if valid_field "$info_value" "$info_kind"; then
            printf '%s=%s\n' "$info_label" "$info_value"
        else
            printf '%s=invalid\n' "$info_label"; info_status=invalid
        fi
    }

    parse_info() {
        info_file=$1
        info_version=; info_version_count=0
        info_full_version=; info_full_version_count=0
        info_flags=; info_flags_count=0
        info_features=; info_features_count=0
        info_uapi_version=; info_uapi_version_count=0
        info_lkm=; info_lkm_count=0
        info_bundled=; info_bundled_count=0
        info_late_load=; info_late_load_count=0
        info_runtime_mode=; info_runtime_mode_count=0
        info_pr_build=; info_pr_build_count=0
        while IFS= read -r info_line; do
            case "$info_line" in
                'version: '*) info_version=${info_line#version: }; info_version_count=$((info_version_count + 1)) ;;
                'full_version: '*) info_full_version=${info_line#full_version: }; info_full_version_count=$((info_full_version_count + 1)) ;;
                'flags: '*) info_flags=${info_line#flags: }; info_flags_count=$((info_flags_count + 1)) ;;
                'features: '*) info_features=${info_line#features: }; info_features_count=$((info_features_count + 1)) ;;
                'uapi_version: '*) info_uapi_version=${info_line#uapi_version: }; info_uapi_version_count=$((info_uapi_version_count + 1)) ;;
                'lkm: '*) info_lkm=${info_line#lkm: }; info_lkm_count=$((info_lkm_count + 1)) ;;
                'bundled: '*) info_bundled=${info_line#bundled: }; info_bundled_count=$((info_bundled_count + 1)) ;;
                'late_load: '*) info_late_load=${info_line#late_load: }; info_late_load_count=$((info_late_load_count + 1)) ;;
                'runtime_mode: '*) info_runtime_mode=${info_line#runtime_mode: }; info_runtime_mode_count=$((info_runtime_mode_count + 1)) ;;
                'pr_build: '*) info_pr_build=${info_line#pr_build: }; info_pr_build_count=$((info_pr_build_count + 1)) ;;
            esac
        done < "$info_file"

        info_status=complete
        emit_info_field info_version "$info_version" "$info_version_count" number
        emit_info_field info_full_version "$info_full_version" "$info_full_version_count" full
        emit_info_field info_flags "$info_flags" "$info_flags_count" flags
        emit_info_field info_features "$info_features" "$info_features_count" flags
        emit_info_field info_uapi_version "$info_uapi_version" "$info_uapi_version_count" number
        emit_info_field info_lkm "$info_lkm" "$info_lkm_count" bool
        emit_info_field info_bundled "$info_bundled" "$info_bundled_count" bool
        emit_info_field info_late_load "$info_late_load" "$info_late_load_count" bool
        emit_info_field info_runtime_mode "$info_runtime_mode" "$info_runtime_mode_count" token
        emit_info_field info_pr_build "$info_pr_build" "$info_pr_build_count" bool
        printf 'info_status=%s\n' "$info_status"
    }

    verified_manager() {
        manager_hash=
        [ -f "$manager_path" ] && [ ! -L "$manager_path" ] && [ -x "$manager_path" ] || return 1
        [ "$(stat -c '%u:%g:%a' "$manager_path" 2>/dev/null)" = 0:0:755 ] || return 1
        manager_hash=$(timeout -s KILL 5 /system/bin/sha256sum "$manager_path" 2>/dev/null) || return 1
        [ "${manager_hash%% *}" = "$manager_sha" ]
    }

    query_info() {
        info_result=unavailable
        info_status=unavailable
        info_tmp=$attempt/.info.tmp
        if timeout -s KILL 5 "$manager_path" debug info > "$info_tmp" 2>/dev/null; then
            info_result=ok
        else
            info_rc=$?
            if [ "$info_rc" -eq 137 ]; then info_result=timeout; fi
        fi
        if [ "$info_result" = ok ]; then
            info_size=$(stat -c '%s' "$info_tmp" 2>/dev/null) || info_size=
            case "$info_size" in
                ''|*[!0123456789]*) info_result=invalid ;;
                *) if [ "$info_size" -gt 4096 ]; then info_result=oversize; fi ;;
            esac
        fi
        if [ "$info_result" = ok ]; then
            parse_info "$info_tmp"
        fi
        rm -f "$info_tmp"
    }

    observe_kernel_version() {
        kernel_version_state='unset'
        kernel_version_value=
        if [ "${KSU_KERNEL_VER_CODE+x}" != x ]; then
            kernel_version_state='unset'
        elif [ -z "$KSU_KERNEL_VER_CODE" ]; then
            kernel_version_state=empty
        else
            case "$KSU_KERNEL_VER_CODE" in
                *[!0123456789]*) kernel_version_state=invalid ;;
                *)
                    if [ "${#KSU_KERNEL_VER_CODE}" -le 10 ]; then
                        kernel_version_state=numeric
                        kernel_version_value=$KSU_KERNEL_VER_CODE
                    else
                        kernel_version_state=invalid
                    fi
                    ;;
            esac
        fi
        printf 'kernel_version_state=%s\n' "$kernel_version_state"
        if [ "$kernel_version_state" = numeric ]; then
            printf 'kernel_version_value=%s\n' "$kernel_version_value"
        fi
    }

    read_kernel_identity() {
        kernel_identity=unavailable
        if [ -r /proc/version ]; then
            if IFS= read -r kernel_line < /proc/version; then
                kernel_identity=mismatch
                case "$kernel_line" in
                    *"$expected_release"*"$expected_build"*) kernel_identity=verified ;;
                esac
            fi
        fi
    }

    read_modules_identity() {
        modules_identity=unavailable
        if [ -r /proc/modules ]; then
            modules_identity=verified
            while IFS=' ' read -r module_name _; do
                [ -n "$module_name" ] || continue
                case "$module_name" in
                    kernelsu|ksu|resukisu|susfs) modules_identity=conflict; break ;;
                esac
            done < /proc/modules
        fi
    }

    collect_identity() {
        manager_identity=unverified
        driver_identity=not_checked
        driver_reason=
        info_result=unavailable
        info_status=unavailable
        kernel_identity=unavailable
        modules_identity=unavailable

        observe_kernel_version
        if verified_manager; then
            manager_identity=verified
            printf 'manager_identity=verified\n'
            query_info
            if [ "$info_result" != ok ]; then
                driver_identity=unavailable
            elif [ "$info_status" != complete ]; then
                driver_identity=invalid
            elif [ "$info_flags" != "$expected_flags" ]; then
                driver_identity=mismatch; driver_reason=flags
            elif [ "$info_features" != "$expected_features" ]; then
                driver_identity=mismatch; driver_reason=features
            elif [ "$info_uapi_version" != 6 ]; then
                driver_identity=mismatch; driver_reason=uapi_version
            elif [ "$info_lkm" != false ]; then
                driver_identity=mismatch; driver_reason=lkm
            elif [ "$info_bundled" != false ]; then
                driver_identity=mismatch; driver_reason=bundled
            elif [ "$info_late_load" != false ]; then
                driver_identity=mismatch; driver_reason=late_load
            elif [ "$info_runtime_mode" != built-in ]; then
                driver_identity=mismatch; driver_reason=runtime_mode
            elif [ "$info_pr_build" != false ]; then
                driver_identity=mismatch; driver_reason=pr_build
            else
                driver_identity=verified
            fi
        else
            printf 'manager_identity=unverified\n'
        fi
        read_kernel_identity
        read_modules_identity

        printf 'info_result=%s\n' "$info_result"
        printf 'driver_identity=%s\n' "$driver_identity"
        if [ -n "$driver_reason" ]; then
            printf 'driver_reason=%s\n' "$driver_reason"
        fi
        printf 'kernel_identity=%s\n' "$kernel_identity"
        printf 'modules_identity=%s\n' "$modules_identity"
    }

    if guard ksu "${KSU:-}" true &&
       guard bakasu "${BAKASU:-}" true &&
       guard module "${KSU_MODULE:-}" earkey_control_preflight &&
       guard userspace_version "${KSU_VER_CODE:-}" 35257 &&
       guard uapi_version "${KSU_UAPI_VER:-}" 6 &&
       guard uid "$(id -u)" 0; then
        collect_identity
        failure=
        if [ "$manager_identity" != verified ]; then
            failure=manager_identity
        elif [ "$driver_identity" != verified ]; then
            failure=driver_identity
        elif [ "$kernel_identity" != verified ]; then
            failure=kernel_identity
        elif [ "$modules_identity" != verified ]; then
            failure=module_identity
        fi
        if [ -n "$failure" ]; then
            printf 'guard_failure=%s\n' "$failure"
        elif [ "$stage" = boot_completed ]; then
            status=0
        elif [ -f ./probe.sha256 ] && [ ! -L ./probe.sha256 ] &&
           [ "$(stat -c '%s' ./probe.sha256)" = 65 ] &&
           [ -f ./earkey-probe ] && [ ! -L ./earkey-probe ] && [ -x ./earkey-probe ] &&
           IFS= read -r expected < ./probe.sha256 &&
           actual=$(timeout -s KILL 5 /system/bin/sha256sum ./earkey-probe) &&
           [ "${actual%% *}" = "$expected" ]; then
            printf 'launcher=resukisu_service\n'
            printf 'artifact_sha256=%s\n' "$expected"
            status=0
            timeout -s KILL 5 ./earkey-probe probe-uinput || status=$?
        else
            printf 'preflight=digest_mismatch\n'
        fi
    fi
    if [ "$stage" = boot_completed ]; then
        printf 'guard_exit=%s\n' "$status"
    else
        printf 'service_probe_exit=%s\n' "$status"
    fi
) > "$attempt/result.tmp" 2>&1
mv "$attempt/result.tmp" "$attempt/result"
